#![feature(once_cell_get_mut)]

mod greetd;
mod sessions;
mod tailwind_colors;

use std::cell::OnceCell;
use std::sync::Arc;

use greetd::client::Client;
use greetd::state::AuthMessageType;
use greetd::transport::{GreetdTransport, MockTransport, Transport};
use greetd::AnyClient;
use iced::theme::{Custom, Palette};
use iced::widget::{
    button, center, column, container, pick_list, svg, text_input, Column, Text, TextInput,
};
use iced::{
    keyboard, widget, Alignment, Background, Border, Color, Element, Length, Subscription, Task,
    Theme,
};
use sessions::Session;

struct Greeter<T: Transport> {
    answered_questions: Vec<String>,
    value: String,
    sessions: OnceCell<Vec<Session>>,
    session: Option<Session>,
    client: Option<AnyClient<T>>,
}

impl<T: Transport> Default for Greeter<T> {
    fn default() -> Self {
        Self {
            answered_questions: Default::default(),
            value: Default::default(),
            sessions: Default::default(),
            session: Default::default(),
            client: Some(Client::new().unwrap().into()),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ValueChanged(String),
    TabPressed { shift: bool },
    SubmitPressed,
    SessionSelected(Session),
}

pub fn main() -> iced::Result {
    match std::env::var("OCF_GREETER_MOCK").ok() {
        Some(_) => run::<MockTransport>(),
        None => run::<GreetdTransport>(),
    }
}

pub fn run<T: Transport + 'static>() -> iced::Result {
    iced::application(Greeter::<T>::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .theme(Greeter::theme)
        .run()
}

impl<T: Transport> Greeter<T> {
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
            Message::TabPressed { shift: false } => widget::focus_next(),
            Message::TabPressed { shift: true } => widget::focus_previous(),
            Message::SessionSelected(session) => {
                self.session = Some(session);
                Task::none()
            }
            Message::SubmitPressed => self.submit(),
        }
    }

    fn submit(&mut self) -> Task<Message> {
        let client = match std::mem::take(&mut self.client).unwrap() {
            AnyClient::Empty(client) => client.create_session(self.value.clone()).unwrap(),
            AnyClient::NeedAuthResponse(client) => {
                client.post_auth_message_response(Some(self.value.clone())).unwrap()
            }
            AnyClient::SessionCreated(client) => {
                client
                    .start_session(
                        self.session.as_ref().unwrap().exec.clone(),
                        self.session.as_ref().unwrap().to_environment(),
                    )
                    .unwrap();
                return Task::none();
            }
            client @ AnyClient::SessionStarted(_) => client,
        };
        self.client = Some(client);

        self.answered_questions.push(std::mem::take(&mut self.value));
        text_input::focus("value")
    }

    fn logo(&self) -> Element<'_, Message> {
        svg("logo.svg").width(96).into()
    }

    fn text_input<'a>(&self, placeholder: &'a str, value: &'a str) -> TextInput<'a, Message> {
        text_input(placeholder, value).padding([8, 16]).style(text_input_style)
    }

    fn login_form(&self) -> Element<'_, Message> {
        let (auth_message, auth_message_type) = match self.client.as_ref().unwrap() {
            AnyClient::Empty(_) => ("Username", AuthMessageType::Visible),
            AnyClient::NeedAuthResponse(client) => {
                (client.state.auth_message.as_str(), client.state.auth_message_type)
            }
            AnyClient::SessionCreated(_) => return column![].into(),
            AnyClient::SessionStarted(_) => return column![].into(),
        };

        let answered_question_inputs =
            self.answered_questions.iter().map(|value| self.text_input("", value).into());

        Column::from_iter(answered_question_inputs)
            .push(
                self.text_input(auth_message, &self.value)
                    .id("value")
                    .on_input(Message::ValueChanged)
                    .on_submit(Message::SubmitPressed)
                    .secure(matches!(auth_message_type, AuthMessageType::Secret)),
            )
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

    fn view(&self) -> Element<'_, Message> {
        center(
            column![self.logo(), self.login_form(), self.submit_button(), self.session_selector()]
                .align_x(Alignment::Center)
                .spacing(24)
                .max_width(384),
        )
        .into()
    }
}

// fn process_create_session_response(state: FormState, response: AnyClient) -> AnyGreeter {
//     match response {
//         AnyClient::EmptyClient(client) => EmptyGreeter { state, client }.into(),
//         AnyClient::NeedAuthResponseClient(client) => match client.state() {
//             NeedAuthResponse { auth_message_type: AuthMessageType::Secret, auth_message }
//                 if auth_message.to_lowercase().contains("password") =>
//             {
//                 let response =
//                     client.post_auth_message_response(Some(state.password.clone())).unwrap();
//                 process_create_session_response(state, response)
//             }
//             _ => NeedAuthResponseGreeter { state, value: Default::default(), client }.into(),
//         },
//         AnyClient::SessionCreatedClient(client) => {
//             let response = client
//                 .start_session(state.session.as_ref().unwrap().exec.clone(), Vec::new())
//                 .unwrap();
//             process_create_session_response(state, response)
//         }
//         AnyClient::SessionStartedClient(_) => EmptyGreeter::default().into(),
//     }
// }

// impl GreeterTrait for EmptyGreeter {
//     fn state(&self) -> &FormState {
//         &self.state
//     }

//     fn state_mut(&mut self) -> &mut FormState {
//         &mut self.state
//     }

//     fn update(mut self, message: Message) -> (Task<Message>, AnyGreeter) {
//         match &message {
//             Message::SubmitPressed => {
//                 let response = self.client.create_session(self.state.username.clone()).unwrap();
//                 (Task::none(), process_create_session_response(self.state, response))
//             }
//             _ => (FormState::update(self.state_mut(), message), self.into()),
//         }
//     }

//     fn view(&self) -> Vec<Element<'_, Message>> {
//         vec![
//             self.state.logo(),
//             self.state.login_form(),
//             self.state.submit_button(),
//             self.state.session_selector(),
//         ]
//     }
// }

// struct NeedAuthResponseGreeter {
//     state: FormState,
//     value: String,
//     client: AnyNeedAuthResponseClient,
// }

// impl GreeterTrait for NeedAuthResponseGreeter {
//     fn state(&self) -> &FormState {
//         &self.state
//     }

//     fn state_mut(&mut self) -> &mut FormState {
//         &mut self.state
//     }

//     fn update(mut self, message: Message) -> (Task<Message>, AnyGreeter) {
//         match message {
//             Message::ValueChanged(value) => {
//                 self.value = value;
//                 (Task::none(), self.into())
//             }
//             Message::SubmitPressed => {
//                 let response = self.client.post_auth_message_response(Some(self.value)).unwrap();
//                 (Task::none(), process_create_session_response(self.state, response))
//             }
//             _ => (FormState::update(self.state_mut(), message), self.into()),
//         }
//     }

//     fn view(&self) -> Vec<Element<'_, Message>> {
//         vec![
//             self.state.logo(),
//             self.state
//                 .text_input(&self.client.state().auth_message, &self.value)
//                 .secure(matches!(self.client.state().auth_message_type, AuthMessageType::Secret))
//                 .on_input(Message::ValueChanged)
//                 .on_submit(Message::SubmitPressed)
//                 .into(),
//             self.state.submit_button(),
//         ]
//     }
// }

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
