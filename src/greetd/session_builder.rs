//! Abstraction over [`greetd::transport::Transport`].
//!
//! There are some requirements on how you can use the greetd transport. For
//! instance, one has to first create a session, then answer any auth messages,
//! then start the session.
//!
//! The [`Client`] struct encodes this logic into the type system.

use std::fmt::Debug;

use color_eyre::eyre::{bail, Result};
use greetd_ipc::{AuthMessageType, Response};

use super::transport::Transport;

#[derive(Debug)]
pub enum AnsweredQuestion {
    Visible(String),
    Secret(String),
}

#[derive(Debug)]
pub struct NeedAuthResponse<T: Transport> {
    pub auth_message_type: greetd_ipc::AuthMessageType,
    pub auth_message: String,
    pub prev_answers: Vec<AnsweredQuestion>,
    transport: T,
}

#[derive(Debug)]
pub struct SessionCreated<T: Transport> {
    pub prev_answers: Vec<AnsweredQuestion>,
    transport: T,
}

#[derive(Debug)]
pub enum SessionBuilder<T: Transport> {
    NeedAuthResponse(NeedAuthResponse<T>),
    SessionCreated(SessionCreated<T>),
}

/// [`create_session`] and [`post_auth_message_response`] handle the responses
/// from greetd in a very similar way — either the session was created
/// successfully, or there is an auth message to respond to.
///
/// This logic is factored into this function.
fn handle_auth_message_response<T>(
    mut transport: T,
    prev_answers: Vec<AnsweredQuestion>,
    response: Response,
) -> Result<SessionBuilder<T>>
where
    T: Transport,
{
    Ok(match response {
        Response::Success => {
            SessionBuilder::SessionCreated(SessionCreated { transport, prev_answers })
        }
        Response::AuthMessage { auth_message_type, auth_message } => {
            SessionBuilder::NeedAuthResponse(NeedAuthResponse {
                auth_message_type,
                auth_message,
                transport,
                prev_answers,
            })
        }
        Response::Error { error_type, description } => {
            transport.cancel_session()?;
            bail!("{:?}: {}", error_type, description)
        }
    })
}

/// Sends a request to create a session.
///
/// When successful, this function returns an enum type for the two cases:
/// - The session was created successfully.
/// - There is an auth message to be answered.
pub fn create_session<T: Transport>(username: String) -> Result<SessionBuilder<T>> {
    let mut transport = T::new()?;
    let response = transport.create_session(username.clone())?;
    handle_auth_message_response(transport, vec![AnsweredQuestion::Visible(username)], response)
}

impl<T: Transport> NeedAuthResponse<T> {
    /// Posts a response to an auth message received from greetd.
    ///
    /// When successful, this function returns an [`Either`] type for the two cases:
    /// - The session was created successfully.
    /// - There is an auth message to be answered.
    pub fn post_auth_message_response(
        mut self,
        response: Option<String>,
    ) -> Result<SessionBuilder<T>> {
        if let Some(response) = response.as_ref() {
            use AnsweredQuestion::*;
            match self.auth_message_type {
                AuthMessageType::Secret => self.prev_answers.push(Secret(response.clone())),
                AuthMessageType::Visible => self.prev_answers.push(Visible(response.clone())),
                _ => {}
            };
        };
        let response = self.transport.post_auth_message_response(response)?;
        handle_auth_message_response(self.transport, self.prev_answers, response)
    }
}

impl<T: Transport> SessionCreated<T> {
    /// Starts the session with the given command and environment. If the request is
    /// successful, the session will be started when the greeter process exits.
    pub fn start_session(mut self, cmd: Vec<String>, env: Vec<String>) -> Result<()> {
        let response = self.transport.start_session(cmd, env)?;

        match response {
            Response::Success => Ok(()),
            Response::Error { error_type, description } => {
                self.transport.cancel_session()?;
                bail!("{:?}: {}", error_type, description)
            }
            Response::AuthMessage { .. } => bail!("unexpected auth_message after start_session"),
        }
    }
}
