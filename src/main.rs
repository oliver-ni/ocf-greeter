mod greetd;
mod sessions;
mod tailwind_colors;

use std::fmt::Debug;
use std::sync::Arc;

use clap::Parser;
use color_eyre::eyre::{bail, Result};
use greetd::session_builder::{self, SessionBuilder};
use greetd::transport::{GreetdTransport, MockTransport, Transport};
use greetd_ipc::AuthMessageType;
use iced::theme::{Custom, Palette};
use iced::widget::svg::Handle;
use iced::widget::{
    button, center, column, container, pick_list, stack, svg, text, text_input, Column, Text,
    TextInput,
};
use iced::{
    keyboard, widget, Alignment, Background, Border, Color, Element, Length, Subscription, Task,
    Theme,
};
use sessions::Session;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Default session name
    #[arg(long)]
    default_session: Option<String>,
}

enum AnsweredQuestion {
    Visible(String),
    Secret(String),
}

struct Greeter<T: Transport> {
    prev_answers: Vec<AnsweredQuestion>,
    value: String,
    sessions: Vec<Session>,
    session: Option<Session>,
    error_message: Option<String>,
    session_builder: Option<SessionBuilder<T>>,
}

impl<T: Transport> Default for Greeter<T> {
    fn default() -> Self {
        Self {
            prev_answers: Default::default(),
            value: Default::default(),
            sessions: sessions::get_sessions(),
            session: Default::default(),
            error_message: Default::default(),
            session_builder: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ValueChanged(String),
    SessionSelected(Session),
    TabPressed { shift: bool },
    SubmitPressed,
}

pub fn main() -> iced::Result {
    let args = Args::parse();

    match std::env::var("OCF_GREETER_MOCK").ok() {
        Some(_) => run::<MockTransport>(args),
        None => run::<GreetdTransport>(args),
    }
}

fn run<T: Transport + Debug + 'static>(args: Args) -> iced::Result {
    let mut state = Greeter::<T>::default();

    match args.default_session {
        Some(slug) => state.session = state.sessions.iter().find(|s| s.slug == slug).cloned(),
        None => {}
    };

    iced::application(Greeter::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .theme(Greeter::theme)
        .run_with(|| (state, Task::none()))
}

impl<T: Transport + Debug> Greeter<T> {
    fn title(&self) -> String {
        "Welcome to the Open Computing Facility!".to_owned()
    }

    fn theme(&self) -> Theme {
        let palette = Palette {
            background: tailwind_colors::GRAY_100,
            text: Color::BLACK,
            primary: tailwind_colors::SKY_950,
            success: tailwind_colors::GREEN_500,
            danger: tailwind_colors::RED_500,
        };

        Theme::Custom(Arc::new(Custom::new("OCF".to_owned(), palette)))
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::Named::Tab;
        use keyboard::Key;

        keyboard::on_key_press(|key, modifiers| match key {
            Key::Named(Tab) => Some(Message::TabPressed { shift: modifiers.shift() }),
            _ => None,
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ValueChanged(value) => {
                self.value = value;
                Task::none()
            }
            Message::SessionSelected(session) => {
                self.session = Some(session);
                Task::none()
            }
            Message::TabPressed { shift: false } => widget::focus_next(),
            Message::TabPressed { shift: true } => widget::focus_previous(),

            Message::SubmitPressed => match self.submit() {
                Ok(task) => {
                    self.error_message = None;
                    task
                }
                Err(error) => {
                    self.error_message = Some(error.to_string());
                    self.prev_answers.clear();
                    Task::none()
                }
            },
        }
    }

    fn submit(&mut self) -> Result<Task<Message>> {
        Ok(match std::mem::take(&mut self.session_builder) {
            None => {
                let value = std::mem::take(&mut self.value);
                self.prev_answers.push(AnsweredQuestion::Visible(value.clone()));
                self.session_builder = Some(session_builder::create_session(value)?);
                text_input::focus("value")
            }

            Some(SessionBuilder::NeedAuthResponse(builder)) => {
                let value = std::mem::take(&mut self.value);
                self.prev_answers.push(match builder.auth_message_type {
                    AuthMessageType::Secret => AnsweredQuestion::Secret(value.clone()),
                    AuthMessageType::Visible => AnsweredQuestion::Visible(value.clone()),
                    _ => todo!(),
                });
                self.session_builder = Some(builder.post_auth_message_response(Some(value))?);

                // Automatically try to start the session
                Task::done(Message::SubmitPressed)
            }

            Some(SessionBuilder::SessionCreated(builder)) => {
                let session = match self.session.as_ref() {
                    Some(session) => session,
                    None => {
                        self.session_builder = Some(SessionBuilder::SessionCreated(builder));
                        bail!("No session selected");
                    }
                };
                builder.start_session(session.exec.clone(), session.to_environment())?;

                #[cfg(target_os = "linux")]
                // Iced currently has a bug where exiting normally on Wayland
                // causes a segfault. WTF? So we immediately exit for now.
                // https://github.com/iced-rs/iced/issues/2625
                std::process::exit(0);

                iced::exit()
            }
        })
    }

    fn logo(&self) -> Element<'_, Message> {
        svg(Handle::from_memory(include_bytes!("logo.svg"))).width(96).into()
    }

    fn text_input<'a>(&self, placeholder: &'a str, value: &'a str) -> TextInput<'a, Message> {
        text_input(placeholder, value).padding([8, 16]).style(text_input_style)
    }

    fn login_form(&self) -> Element<'_, Message> {
        let (auth_message, auth_message_type) = match &self.session_builder {
            None => ("Username", &AuthMessageType::Visible),
            Some(SessionBuilder::NeedAuthResponse(builder)) => {
                // Remove colon at the end of the description, if it exists
                let description = builder.auth_message.as_str().trim().trim_end_matches(":");
                (description, &builder.auth_message_type)
            }
            Some(SessionBuilder::SessionCreated(_)) => return column![].into(),
        };

        let answered_question_inputs = self.prev_answers.iter().map(|value| match value {
            AnsweredQuestion::Visible(value) => self.text_input("", &value).into(),
            AnsweredQuestion::Secret(value) => self.text_input("", &value).secure(true).into(),
        });

        let next_input = self
            .text_input(auth_message, &self.value)
            .id("value")
            .on_input(Message::ValueChanged)
            .on_submit(Message::SubmitPressed)
            .secure(matches!(auth_message_type, AuthMessageType::Secret));

        Column::from_iter(answered_question_inputs)
            .push(next_input)
            .spacing(12)
            .align_x(Alignment::Center)
            .into()
    }

    fn submit_button(&self) -> Element<'_, Message> {
        let button = |value| {
            button(Text::new(value).width(Length::Fill).center())
                .padding([8, 16])
                .width(Length::Fill)
                .style(button_style)
        };
        button("Submit").on_press(Message::SubmitPressed).into()
    }

    fn session_selector(&self) -> Element<'_, Message> {
        container(
            pick_list(self.sessions.as_slice(), self.session.clone(), Message::SessionSelected)
                .placeholder("choose a session"),
        )
        .width(Length::Fill)
        .align_x(Alignment::End)
        .into()
    }

    fn error_message(&self) -> Option<Element<'_, Message>> {
        self.error_message
            .as_ref()
            .map(|message| text!("{}", message).color(tailwind_colors::RED_500).center().into())
    }

    fn view(&self) -> Element<'_, Message> {
        stack![
            center(
                column![self.logo(), self.login_form(), self.submit_button()]
                    .push_maybe(self.error_message())
                    .align_x(Alignment::Center)
                    .spacing(24)
                    .max_width(384),
            ),
            container(self.session_selector())
                .padding(10)
                .align_right(Length::Fill)
                .align_bottom(Length::Fill),
        ]
        .into()
    }
}

fn button_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: Border { radius: 3.into(), ..Default::default() },
        ..button::primary(theme, status)
    }
}

fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(tailwind_colors::GRAY_300),
        border: Border { radius: 3.into(), ..Default::default() },
        placeholder: tailwind_colors::GRAY_400,
        ..text_input::default(theme, status)
    }
}
