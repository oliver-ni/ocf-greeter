use std::fmt::Debug;

use greetd_ipc::Request::*;
use greetd_ipc::Response;

use super::state::ErrorEncountered;
use super::transport::Transport;
use super::{AnyClient, Empty, NeedAuthResponse, SessionCreated, SessionStarted};

pub struct Client<State, T: Transport> {
    pub state: State,
    transport: T,
}

impl<State: Debug, T: Debug + Transport> Debug for Client<State, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("state", &self.state)
            .field("transport", &self.transport)
            .finish()
    }
}

impl<State, T: Transport> Client<State, T> {
    fn transition<NewState>(self, state: NewState) -> Client<NewState, T> {
        Client { state, transport: self.transport }
    }

    fn handle_response(
        self,
        response: Response,
        on_success: impl FnOnce(Self) -> AnyClient<T>,
    ) -> Result<AnyClient<T>, T::Error> {
        let client = match response {
            Response::Success => on_success(self),
            Response::AuthMessage { auth_message_type, auth_message } => {
                self.transition(NeedAuthResponse { auth_message_type, auth_message }).into()
            }
            Response::Error { error_type, description } => {
                self.transition(ErrorEncountered { error_type, description }).into()
            }
        };
        Ok(client)
    }

    pub fn cancel_session(mut self) -> Result<AnyClient<T>, T::Error> {
        let response = self.transport.send_request(CancelSession)?;
        self.handle_response(response, |client| client.transition(Empty).into())
    }
}

impl<T: Transport> Client<Empty, T> {
    pub fn new() -> Result<Self, T::Error> {
        Ok(Self { transport: T::new()?, state: Empty })
    }

    pub fn create_session(mut self, username: String) -> Result<AnyClient<T>, T::Error> {
        let response = self.transport.send_request(CreateSession { username })?;
        self.handle_response(response, |client| client.transition(SessionCreated).into())
    }
}

impl<T: Transport> Client<NeedAuthResponse, T> {
    pub fn post_auth_message_response(
        mut self,
        response: Option<String>,
    ) -> Result<AnyClient<T>, T::Error> {
        let response = self.transport.send_request(PostAuthMessageResponse { response })?;
        self.handle_response(response, |client| client.transition(SessionCreated).into())
    }
}

impl<T: Transport> Client<SessionCreated, T> {
    pub fn start_session(
        mut self,
        cmd: Vec<String>,
        env: Vec<String>,
    ) -> Result<AnyClient<T>, T::Error> {
        let response = self.transport.send_request(StartSession { cmd, env })?;
        self.handle_response(response, |client| client.transition(SessionStarted).into())
    }
}
