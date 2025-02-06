mod components;
mod greetd;
mod sessions;

use color_eyre::eyre::{bail, Result};
use components::{Button, Input, SessionSelector};
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use greetd::session_builder::{
    self, AnsweredQuestion, NeedAuthResponse, SessionBuilder, SessionCreated,
};
use greetd::transport::{GreetdTransport, MockTransport, Transport};
use greetd_ipc::AuthMessageType;
use sessions::Session;

const LOGO: Asset = asset!("/assets/logo.svg");
const BACKGROUND: Asset = asset!("/assets/login.png");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    let config = dioxus::LaunchBuilder::new().with_cfg(
        Config::default()
            .with_menu(None)
            .with_window(WindowBuilder::new().with_maximized(true).with_decorations(false)),
    );

    match std::env::var("OCF_GREETER_MOCK").ok() {
        Some(_) => config.launch(App::<MockTransport>),
        None => config.launch(App::<GreetdTransport>),
    }
}

fn get_sessions() -> Vec<Session> {
    match std::env::var("OCF_GREETER_MOCK").ok() {
        Some(_) => sessions::get_sessions_mock(),
        None => sessions::get_sessions(),
    }
}

struct State<T: Transport> {
    session_builder: Option<SessionBuilder<T>>,
    value: String,
    sessions: Vec<Session>,
    session: Option<Session>,
}

impl<T: Transport> Default for State<T> {
    fn default() -> Self {
        let sessions = get_sessions();

        let session = std::env::var("DEFAULT_SESSION")
            .ok()
            .and_then(|slug| sessions.iter().find(|session| session.slug == slug).cloned());

        Self { session_builder: Default::default(), value: Default::default(), sessions, session }
    }
}

impl<T: Transport> State<T> {
    fn submit(&mut self) -> Result<()> {
        Ok(match std::mem::take(&mut self.session_builder) {
            None => {
                let value = std::mem::take(&mut self.value);
                self.session_builder = Some(session_builder::create_session(value)?);
            }

            Some(SessionBuilder::NeedAuthResponse(builder)) => {
                let value = std::mem::take(&mut self.value);
                self.session_builder = Some(builder.post_auth_message_response(Some(value))?);

                // If this auth response led to the session being created, automatically try to start it
                if let Some(SessionBuilder::SessionCreated(_)) = self.session_builder {
                    return self.submit();
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
                std::process::exit(0);
            }
        })
    }
}

#[component]
fn App<T: Transport + 'static>() -> Element {
    let mut state = use_signal(|| State::<T>::default());
    let mut error_message = use_signal(|| None);

    let oninput_value = move |event: FormEvent| state.write().value = event.value();
    let onchange_session = move |session: Option<Session>| state.write().session = session;

    let onsubmit = move |event: FormEvent| {
        event.prevent_default();
        match state.write().submit() {
            Ok(()) => error_message.set(None),
            Err(error) => error_message.set(Some(error.to_string())),
        };
    };

    let answered_question_inputs: Vec<_> = {
        // Previously answered text inputs

        let state_value = state.read();

        let prev_answers = match &state_value.session_builder {
            None => &[][..],
            Some(
                SessionBuilder::NeedAuthResponse(NeedAuthResponse { prev_answers, .. })
                | SessionBuilder::SessionCreated(SessionCreated { prev_answers, .. }),
            ) => &prev_answers[..],
        };

        prev_answers
            .iter()
            .map(|value| match value {
                AnsweredQuestion::Visible(value) => (value, false),
                AnsweredQuestion::Secret(value) => (value, true),
            })
            .map(|(value, secure)| {
                rsx!(Input {
                    placeholder: "",
                    value: value,
                    secure: secure,
                    disabled: true,
                    oninput: |_| {}
                })
            })
            .collect()
    };

    let next_input = {
        // The currently active text input.

        let description_and_secure = match &state.read().session_builder {
            None => Some(("Username".to_owned(), false)),
            Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                auth_message_type: AuthMessageType::Visible,
                auth_message,
                ..
            })) => Some((auth_message.clone(), false)),
            Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                auth_message_type: AuthMessageType::Secret,
                auth_message,
                ..
            })) => Some((auth_message.clone(), true)),
            _ => None,
        };

        description_and_secure.map(|(description, secure)| {
            rsx! {
                Input {
                    placeholder: description.trim().trim_end_matches(":"),
                    value: &state.read().value,
                    secure: secure,
                    oninput: oninput_value
                }
            }
        })
    };

    let info_message = {
        let message = match &state.read().session_builder {
            Some(SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                auth_message_type: AuthMessageType::Info | AuthMessageType::Error,
                auth_message,
                ..
            })) => Some(auth_message.clone()),
            _ => None,
        };

        message.map(|message| {
            rsx!(p {
                class: "text-center",
                {message}
            })
        })
    };

    let error_message = error_message().map(|message| {
        rsx!(p {
            class: "text-center text-red-500",
            {message}
        })
    });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        FormWrapper {
            onsubmit: onsubmit,
            {answered_question_inputs.iter()}
            {next_input}
            {info_message}
            Button { "Submit" }
            {error_message}
            SessionSelector {
                sessions: state.read().sessions.clone(),
                session: state.read().session.clone(),
                onchange: onchange_session
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct FormWrapperProps {
    onsubmit: EventHandler<FormEvent>,
    children: Element,
}

#[component]
pub fn FormWrapper(props: FormWrapperProps) -> Element {
    rsx! {
        div {
            class: "h-full bg-center bg-cover flex flex-col items-center justify-center gap-4",
            background_image: format!("url({})", BACKGROUND),
            img { src: LOGO, class: "w-20" }
            form {
                onsubmit: props.onsubmit,
                class: "p-4 w-96 flex flex-col gap-4 rounded-lg",
                {props.children}
            }
        }
    }
}
