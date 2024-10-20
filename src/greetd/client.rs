use color_eyre::eyre::{bail, Context, Result};
use greetd_ipc::{Request, Response};

use super::transport::Transport;
use super::{AnyClient, Empty, NeedAuthResponse, SessionCreated, SessionStarted};

pub struct Client<State, T: Transport> {
    pub state: State,
    transport: T,
}

impl<State, T: Transport> Client<State, T> {
    fn handle_response(self, response: Response) -> Result<AnyClient<T>> {
        let transport = self.transport;

        let client = match response {
            Response::Success => Client { state: SessionCreated, transport }.into(),

            Response::AuthMessage { auth_message_type, auth_message } => Client {
                state: NeedAuthResponse {
                    auth_message_type: auth_message_type.into(),
                    auth_message,
                },
                transport,
            }
            .into(),

            Response::Error { description, .. } => {
                bail!("Error: {}", description)
            }
        };

        Ok(client)
    }
}

impl<T: Transport> Client<Empty, T> {
    pub fn new() -> Result<Self> {
        Ok(Self { transport: T::new().wrap_err("test")?, state: Empty })
    }

    pub fn create_session(mut self, username: String) -> Result<AnyClient<T>> {
        let response = self.transport.send_request(Request::CreateSession { username })?;
        self.handle_response(response)
    }
}

impl<T: Transport> Client<NeedAuthResponse, T> {
    pub fn post_auth_message_response(mut self, response: Option<String>) -> Result<AnyClient<T>> {
        let response =
            self.transport.send_request(Request::PostAuthMessageResponse { response })?;
        self.handle_response(response)
    }
}

impl<T: Transport> Client<SessionCreated, T> {
    pub fn start_session(mut self, cmd: Vec<String>, env: Vec<String>) -> Result<AnyClient<T>> {
        let response = self.transport.send_request(Request::StartSession { cmd, env })?;
        self.handle_response(response)
    }

    pub fn cancel_session(mut self) -> Result<AnyClient<T>> {
        let _ = self.transport.send_request(Request::CancelSession)?;
        Ok(Client { state: Empty, transport: self.transport }.into())
    }
}

impl<T: Transport> Client<SessionStarted, T> {}
