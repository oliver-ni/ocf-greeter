#![feature(once_cell_get_mut)]

mod greetd;
mod sessions;
mod tailwind_colors;

use std::cell::OnceCell;
use std::fmt::Debug;
use std::sync::Arc;

use greetd::client::Client;
use greetd::transport::{GreetdTransport, MockTransport, Transport};
use greetd::AnyClient;
use greetd_ipc::AuthMessageType;
use iced::theme::{Custom, Palette};
use iced::widget::svg::Handle;
use iced::widget::{
    button, center, column, container, pick_list, svg, text, text_input, Column, Text, TextInput,
};
use iced::{
    keyboard, widget, Alignment, Background, Border, Color, Element, Length, Subscription, Task,
    Theme,
};
use sessions::Session;
use thiserror::Error;

enum AnsweredQuestion {
    Visible(String),
    Secret(String),
}

struct Greeter<T: Transport> {
    prev_answers: Vec<AnsweredQuestion>,
    value: String,
    sessions: OnceCell<Vec<Session>>,
    session: Option<Session>,
    error_message: Option<String>,
    client: AnyClient<T>,
}

impl<T: Transport> Default for Greeter<T> {
    fn default() -> Self {
        Self {
            prev_answers: Default::default(),
            value: Default::default(),
            sessions: Default::default(),
            session: Default::default(),
            error_message: Default::default(),
            client: Client::new().unwrap().into(),
        }
    }
}

#[derive(Debug, Error)]
enum Error<T: Transport> {
    #[error("Transport error")]
    TransportError(T::Error),

    #[error("No session selected")]
    NoSessionSelected,
}

#[derive(Debug, Clone)]
enum Message {
    ValueChanged(String),
    SessionSelected(Session),
    TabPressed { shift: bool },
    SubmitPressed,
}

pub fn main() -> iced::Result {
    match std::env::var("OCF_GREETER_MOCK").ok() {
        Some(_) => run::<MockTransport>(),
        None => run::<GreetdTransport>(),
    }
}

pub fn run<T: Transport + Debug + 'static>() -> iced::Result {
    iced::application(Greeter::<T>::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .theme(Greeter::theme)
        .run()
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
                Ok(task) => task,
                Err(error) => {
                    self.error_message = Some(error.to_string());
                    self.prev_answers.clear();
                    Task::none()
                }
            },
        }
    }

    fn try_replace_client_with_or_abort(
        client: &mut AnyClient<T>,
        f: impl FnOnce(AnyClient<T>) -> Result<AnyClient<T>, Error<T>>,
    ) -> Result<(), Error<T>> {
        replace_with::replace_with_or_abort_and_return(client, |client| match dbg!(f(client)) {
            Ok(client) => (Ok(()), client),
            Err(error) => (Err(error), Client::new().unwrap().into()),
        })
    }

    fn submit(&mut self) -> Result<Task<Message>, Error<T>> {
        Self::try_replace_client_with_or_abort(&mut self.client, |client| match client {
            AnyClient::Empty(client) => {
                let value = std::mem::take(&mut self.value);
                self.prev_answers.push(AnsweredQuestion::Visible(value.clone()));
                client.create_session(value).map_err(Error::TransportError)
            }

            AnyClient::NeedAuthResponse(client) => {
                let value = std::mem::take(&mut self.value);

                match client.state.auth_message_type {
                    AuthMessageType::Secret => {
                        self.prev_answers.push(AnsweredQuestion::Secret(value.clone()))
                    }
                    AuthMessageType::Visible => {
                        self.prev_answers.push(AnsweredQuestion::Visible(value.clone()))
                    }
                    _ => {}
                };

                client.post_auth_message_response(Some(value)).map_err(Error::TransportError)
            }

            AnyClient::SessionCreated(client) => {
                let session = self.session.as_ref().ok_or(Error::NoSessionSelected)?;
                let _ = client
                    .start_session(session.exec.clone(), session.to_environment())
                    .map_err(Error::TransportError)?;
                todo!()
            }

            AnyClient::ErrorEncountered(client) => {
                client.cancel_session().map_err(Error::TransportError)
            }

            client @ AnyClient::SessionStarted(_) => Ok(client),
        })?;

        Ok(text_input::focus("value"))
    }

    fn logo(&self) -> Element<'_, Message> {
        svg(Handle::from_memory(include_bytes!("logo.svg"))).width(96).into()
    }

    fn text_input<'a>(&self, placeholder: &'a str, value: &'a str) -> TextInput<'a, Message> {
        text_input(placeholder, value).padding([8, 16]).style(text_input_style)
    }

    fn login_form(&self) -> Element<'_, Message> {
        let (auth_message, auth_message_type) = match &self.client {
            AnyClient::Empty(_) => ("Username", &AuthMessageType::Visible),
            AnyClient::NeedAuthResponse(client) => {
                (client.state.auth_message.as_str(), &client.state.auth_message_type)
            }
            AnyClient::SessionCreated(_) => return column![].into(),
            AnyClient::SessionStarted(_) => return column![].into(),
            AnyClient::ErrorEncountered(_) => return column![].into(),
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
        let button_text = match self.client {
            AnyClient::Empty(_) => "Begin",
            AnyClient::NeedAuthResponse(_) => "Continue",
            AnyClient::SessionCreated(_) => "Start Session",
            AnyClient::SessionStarted(_) => "ummmm",
            AnyClient::ErrorEncountered(_) => "Ok",
        };
        button(button_text).on_press(Message::SubmitPressed).into()
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

    fn error_message(&self) -> Option<Element<'_, Message>> {
        self.error_message
            .as_ref()
            .map(|message| text!("{}", message).color(tailwind_colors::RED_500).center().into())
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![self.logo(), self.login_form(), self.submit_button(), self.session_selector()]
                .push_maybe(self.error_message())
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
