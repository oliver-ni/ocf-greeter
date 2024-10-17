#![feature(once_cell_get_mut)]

mod greetd;
mod sessions;
mod tailwind_colors;

use std::cell::OnceCell;
use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use greetd::impl_mock::MockClient;
use greetd::r#impl::GreetdClient;
use greetd::{AnyClient, AnyEmptyClient, AnyNeedAuthResponseClient, EmptyClient};
use iced::theme::{Custom, Palette};
use iced::widget::{
    button, center, column, container, pick_list, svg, text, text_input, Column, Text,
};
use iced::{
    keyboard, widget, Alignment, Background, Border, Color, Element, Length, Subscription, Task,
    Theme,
};
use sessions::Session;

pub fn main() -> iced::Result {
    iced::application(
        GreeterWrapper::title,
        GreeterWrapper::update,
        GreeterWrapper::view,
    )
    .subscription(GreeterWrapper::subscription)
    .theme(GreeterWrapper::theme)
    .run()
}

#[derive(Default)]
struct GreeterWrapper(AnyGreeter);

impl GreeterWrapper {
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
            Key::Named(Tab) => Some(Message::TabPressed {
                shift: modifiers.shift(),
            }),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            Column::with_children(self.0.view())
                .align_x(Alignment::Center)
                .spacing(24)
                .max_width(384),
        )
        .into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let old = std::mem::take(&mut self.0);
        let (task, new) = old.update(message);
        self.0 = new;
        task
    }
}

#[derive(Debug, Clone)]
enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    TabPressed { shift: bool },
    SubmitPressed,
    SessionSelected(Session),
}

#[enum_dispatch(GreeterTrait)]
enum AnyGreeter {
    EmptyGreeter(EmptyGreeter),
    NeedAuthResponseGreeter(NeedAuthResponseGreeter),
    // SessionCreatedGreeter(SessionCreatedGreeter),
    // SessionStartedGreeter(SessionStartedGreeter),
}

impl Default for AnyGreeter {
    fn default() -> Self {
        Self::EmptyGreeter(Default::default())
    }
}

#[derive(Default)]
struct FormState {
    username: String,
    password: String,
    sessions: OnceCell<Vec<Session>>,
    session: Option<Session>,
}

impl FormState {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UsernameChanged(username) => {
                self.username = username;
                Task::none()
            }
            Message::PasswordChanged(password) => {
                self.password = password;
                Task::none()
            }
            Message::TabPressed { shift: false } => widget::focus_next(),
            Message::TabPressed { shift: true } => widget::focus_previous(),
            Message::SessionSelected(session) => {
                self.session = Some(session);
                Task::none()
            }
            Message::SubmitPressed => Task::none(),
        }
    }

    fn logo(&self) -> Element<'_, Message> {
        svg("logo.svg").width(96).into()
    }

    fn login_form(&self) -> Element<'_, Message> {
        let text_input = |placeholder, value| {
            text_input(placeholder, value)
                .padding([8, 16])
                .style(text_input_style)
        };
        column![
            text_input("Username", &self.username)
                .on_input(Message::UsernameChanged)
                .on_submit(Message::SubmitPressed),
            text_input("Password", &self.password)
                .secure(true)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::SubmitPressed)
        ]
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
        button("Login").on_press(Message::SubmitPressed).into()
    }

    fn session_selector(&self) -> Element<'_, Message> {
        container(
            pick_list(
                self.sessions.get_or_init(sessions::get_sessions).as_slice(),
                self.session.clone(),
                Message::SessionSelected,
            )
            .placeholder("choose a session"),
        )
        .width(Length::Fill)
        .align_x(Alignment::End)
        .into()
    }

    fn view(&self) -> Vec<Element<'_, Message>> {
        vec![
            self.logo(),
            self.login_form(),
            self.submit_button(),
            self.session_selector(),
        ]
    }
}

#[enum_dispatch]
trait GreeterTrait {
    fn state(&self) -> &FormState;
    fn state_mut(&mut self) -> &mut FormState;
    fn view(&self) -> Vec<Element<'_, Message>> {
        FormState::view(self.state())
    }
    fn update(self, message: Message) -> (Task<Message>, AnyGreeter);
}

struct EmptyGreeter {
    state: FormState,
    client: AnyEmptyClient,
}

impl Default for EmptyGreeter {
    fn default() -> Self {
        let client: AnyEmptyClient = match std::env::var("OCF_GREETER_MOCK") {
            Err(std::env::VarError::NotPresent) => GreetdClient::new().unwrap().into(),
            _ => MockClient::new().unwrap().into(),
        };
        Self {
            state: Default::default(),
            client,
        }
    }
}

impl GreeterTrait for EmptyGreeter {
    fn state(&self) -> &FormState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut FormState {
        &mut self.state
    }

    fn update(mut self, message: Message) -> (Task<Message>, AnyGreeter) {
        match &message {
            Message::SubmitPressed => (
                Task::none(),
                match self
                    .client
                    .create_session(self.state.username.clone())
                    .unwrap()
                {
                    AnyClient::EmptyClient(client) => EmptyGreeter {
                        state: self.state,
                        client,
                    }
                    .into(),
                    AnyClient::NeedAuthResponseClient(client) => NeedAuthResponseGreeter {
                        state: self.state,
                        client,
                    }
                    .into(),
                    _ => panic!("oops"),
                },
            ),
            _ => (FormState::update(self.state_mut(), message), self.into()),
        }
    }
}

struct NeedAuthResponseGreeter {
    state: FormState,
    client: AnyNeedAuthResponseClient,
}

impl GreeterTrait for NeedAuthResponseGreeter {
    fn state(&self) -> &FormState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut FormState {
        &mut self.state
    }

    fn update(mut self, message: Message) -> (Task<Message>, AnyGreeter) {
        let task = match &message {
            Message::SubmitPressed => Task::none(),
            _ => FormState::update(self.state_mut(), message),
        };
        (task, self.into())
    }

    fn view(&self) -> Vec<Element<'_, Message>> {
        vec![text!("Initiated login").into()]
    }
}

fn button_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: Border {
            radius: 3.into(),
            ..Default::default()
        },
        ..button::primary(theme, status)
    }
}

fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(tailwind_colors::GRAY_300),
        border: Border {
            radius: 3.into(),
            ..Default::default()
        },
        placeholder: tailwind_colors::GRAY_400,
        ..text_input::default(theme, status)
    }
}
