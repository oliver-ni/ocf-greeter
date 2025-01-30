mod greetd;
mod sessions;
mod tailwind_colors;

use std::fmt::Debug;
use std::sync::Arc;

use clap::Parser;
use color_eyre::eyre::{bail, Result};
use greetd::session_builder::{
    self, AnsweredQuestion, NeedAuthResponse, SessionBuilder, SessionCreated,
};
use greetd::transport::{GreetdTransport, MockTransport, Transport};
use greetd_ipc::AuthMessageType;
use iced::theme::{Custom, Palette};
use iced::widget::svg::Handle;
use iced::widget::{
    button, center, column, container, pick_list, row, stack, svg, text, text_input, Column, Text,
    TextInput,
};
use iced::{
    keyboard, time, widget, Alignment, Background, Border, Color, Element, Length, Subscription,
    Task, Theme,
};
use sessions::Session;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Default session name
    #[arg(long)]
    default_session: Option<String>,
}

struct Greeter<T: Transport> {
    now: chrono::DateTime<chrono::Local>,
    value: String,
    sessions: Vec<Session>,
    session: Option<Session>,
    error_message: Option<String>,
    session_builder: Option<SessionBuilder<T>>,
}

impl<T: Transport> Default for Greeter<T> {
    fn default() -> Self {
        Self {
            now: chrono::offset::Local::now(),
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
    Tick(chrono::DateTime<chrono::Local>),
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

    // Focus the initial text input
    let task = text_input::focus("value");

    iced::application(Greeter::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .theme(Greeter::theme)
        .run_with(|| (state, task))
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

        Subscription::batch([
            keyboard::on_key_press(|key, modifiers| match key {
                Key::Named(Tab) => Some(Message::TabPressed { shift: modifiers.shift() }),
                _ => None,
            }),
            time::every(time::Duration::from_millis(500))
                .map(|_| Message::Tick(chrono::offset::Local::now())),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.now = now;
                Task::none()
            }
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
                    Task::none()
                }
            },
        }
    }

    fn submit(&mut self) -> Result<Task<Message>> {
        Ok(match std::mem::take(&mut self.session_builder) {
            None => {
                let value = std::mem::take(&mut self.value);
                self.session_builder = Some(session_builder::create_session(value)?);
                text_input::focus("value")
            }

            Some(SessionBuilder::NeedAuthResponse(builder)) => {
                let value = std::mem::take(&mut self.value);
                self.session_builder = Some(builder.post_auth_message_response(Some(value))?);

                // Automatically try to start the session
                match self.session_builder {
                    Some(SessionBuilder::SessionCreated(_)) => Task::done(Message::SubmitPressed),
                    _ => text_input::focus("value"),
                }
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
        let answered_question_inputs = {
            // Previously answered text inputs

            let prev_answers = match &self.session_builder {
                None => &[][..],
                Some(
                    SessionBuilder::NeedAuthResponse(NeedAuthResponse { prev_answers, .. })
                    | SessionBuilder::SessionCreated(SessionCreated { prev_answers, .. }),
                ) => &prev_answers[..],
            };

            prev_answers.iter().map(|value| match value {
                AnsweredQuestion::Visible(value) => self.text_input("", &value).into(),
                AnsweredQuestion::Secret(value) => self.text_input("", &value).secure(true).into(),
            })
        };

        let next_input = {
            // The currently active text input.

            let description_and_secure = match &self.session_builder {
                None => Some(("Username", false)),
                Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                    auth_message_type: AuthMessageType::Visible,
                    auth_message,
                    ..
                })) => Some((auth_message.as_str(), false)),
                Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                    auth_message_type: AuthMessageType::Secret,
                    auth_message,
                    ..
                })) => Some((auth_message.as_str(), true)),
                _ => None,
            };

            description_and_secure.map(|(description, secure)| {
                self.text_input(
                    // Remove colon at the end of the description, if it exists
                    description.trim().trim_end_matches(":"),
                    &self.value,
                )
                .id("value")
                .on_submit(Message::SubmitPressed)
                .on_input(Message::ValueChanged)
                .secure(secure)
            })
        };

        let info_message = {
            let message = match &self.session_builder {
                Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                    auth_message_type: AuthMessageType::Info | AuthMessageType::Error,
                    auth_message,
                    ..
                })) => Some(auth_message),
                _ => None,
            };

            message.map(|message| text!("{}", message))
        };

        Column::from_iter(answered_question_inputs)
            .push_maybe(next_input)
            .push_maybe(info_message)
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
            pick_list(&self.sessions[..], self.session.clone(), Message::SessionSelected)
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

    fn clock(&self) -> Element<'_, Message> {
        text!("{}", self.now.format("%-I:%M:%S %p"))
            .size(20)
            .center()
            .color(tailwind_colors::GRAY_500)
            .into()
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
            container(row![self.clock(), self.session_selector()].align_y(Alignment::End))
                .padding(10)
                .align_left(Length::Fill)
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
