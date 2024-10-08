mod sessions;
mod tailwind_colors;

use std::cell::OnceCell;
use std::sync::Arc;

use iced::theme::{Custom, Palette};
use iced::widget::{button, center, column, container, pick_list, svg, text_input, Text};
use iced::{
    exit, keyboard, widget, Alignment, Background, Border, Color, Element, Length, Subscription,
    Task, Theme,
};
use sessions::Session;

pub fn main() -> iced::Result {
    iced::application(Greeter::title, Greeter::update, Greeter::view)
        .subscription(Greeter::subscription)
        .theme(Greeter::theme)
        .run()
}

#[derive(Default)]
struct Greeter {
    username: String,
    password: String,
    sessions: OnceCell<Vec<Session>>,
    session: Option<Session>,
}

#[derive(Debug, Clone)]
enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    TabPressed { shift: bool },
    SubmitPressed,
    SessionSelected(Session),
}

impl Greeter {
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
            Message::SubmitPressed => exit(),
            Message::SessionSelected(session) => {
                self.session = Some(session);
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::Named::Tab;
        use keyboard::Key;

        keyboard::on_key_press(|key, modifiers| match key {
            Key::Named(Tab) => Some(Message::TabPressed { shift: modifiers.shift() }),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        // Defaults

        let text_input = |placeholder, value| {
            text_input(placeholder, value).padding([6, 12]).style(Self::text_input_style)
        };

        let button = |value| {
            button(Text::new(value).width(Length::Fill).center())
                .padding([6, 12])
                .width(Length::Fill)
                .style(Self::button_style)
        };

        // Actual UI

        let logo = svg("logo.svg").width(100);

        let login_form = {
            column![
                text_input("Username", &self.username)
                    .on_input(Message::UsernameChanged)
                    .on_submit(Message::SubmitPressed),
                text_input("Password", &self.password)
                    .secure(true)
                    .on_input(Message::PasswordChanged)
                    .on_submit(Message::SubmitPressed)
            ]
            .spacing(10)
            .align_x(Alignment::Center)
        };

        let login_button = button("Login").on_press(Message::SubmitPressed);

        let session_selector = {
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
        };

        center(
            column![logo, login_form, login_button, session_selector]
                .align_x(Alignment::Center)
                .spacing(20)
                .max_width(300),
        )
        .into()
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
}
