use std::os::unix::net::UnixStream;

use color_eyre::eyre::{bail, Context, Result};
use greetd_ipc::codec::SyncCodec;
use greetd_ipc::Request::{self, *};
use greetd_ipc::Response;

use super::{
    AnyClient, Empty, EmptyClient, NeedAuthResponse, NeedAuthResponseClient, SessionCreated,
    SessionCreatedClient, SessionStarted, SessionStartedClient,
};

pub struct GreetdClient<State> {
    socket: UnixStream,
    state: State,
}

fn send_request(socket: &mut UnixStream, request: Request) -> Result<Response> {
    request.write_to(socket).wrap_err("failed to write greetd request")?;
    Response::read_from(socket).wrap_err("failed to read greetd response")
}

fn handle_response(socket: UnixStream, response: Response) -> Result<AnyClient> {
    let client: AnyClient = match response {
        Response::Success => {
            AnyClient::session_created(GreetdClient { socket, state: SessionCreated })
        }
        Response::AuthMessage { auth_message_type, auth_message } => {
            AnyClient::need_auth_response(GreetdClient {
                socket,
                state: NeedAuthResponse {
                    auth_message_type: auth_message_type.into(),
                    auth_message,
                },
            })
        }
        Response::Error { description, .. } => {
            bail!("Error: {}", description)
        }
    };

    Ok(client)
}

impl GreetdClient<Empty> {
    pub fn new() -> Result<Self> {
        let sock = std::env::var("GREETD_SOCK").wrap_err("missing GREETD_SOCK in environment")?;
        let socket = UnixStream::connect(sock).wrap_err("failed to connect to greetd socket")?;
        Ok(Self { socket, state: Empty })
    }
}

impl EmptyClient for GreetdClient<Empty> {
    fn create_session(mut self, username: String) -> Result<AnyClient> {
        let response = send_request(&mut self.socket, CreateSession { username })?;
        handle_response(self.socket, response)
    }

    fn state(&self) -> &Empty {
        &self.state
    }
}

impl NeedAuthResponseClient for GreetdClient<NeedAuthResponse> {
    fn post_auth_message_response(mut self, response: Option<String>) -> Result<AnyClient> {
        let response = send_request(&mut self.socket, PostAuthMessageResponse { response })?;
        handle_response(self.socket, response)
    }

    fn state(&self) -> &NeedAuthResponse {
        &self.state
    }
}

impl SessionCreatedClient for GreetdClient<SessionCreated> {
    fn start_session(mut self, cmd: Vec<String>, env: Vec<String>) -> Result<AnyClient> {
        let response = send_request(&mut self.socket, StartSession { cmd, env })?;
        handle_response(self.socket, response)
    }

    fn cancel_session(mut self) -> Result<AnyClient> {
        let _ = send_request(&mut self.socket, CancelSession)?;
        Ok(AnyClient::empty(GreetdClient { socket: self.socket, state: Empty }))
    }

    fn state(&self) -> &SessionCreated {
        &self.state
    }
}

impl SessionStartedClient for GreetdClient<SessionStarted> {
    fn state(&self) -> &SessionStarted {
        &self.state
    }
}
