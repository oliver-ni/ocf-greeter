use std::os::unix::net::UnixStream;

use color_eyre::eyre::{Context, Result};
use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{Request, Response};

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn new() -> Result<Self> {
        let sock = std::env::var("GREETD_SOCK").wrap_err("missing GREETD_SOCK in environment")?;
        let socket = UnixStream::connect(sock).wrap_err("failed to connect to greetd socket")?;

        Ok(Self { socket })
    }

    pub fn send_request(&mut self, request: greetd_ipc::Request) -> Result<Response> {
        request.write_to(&mut self.socket).wrap_err("failed to write greetd request")?;
        Response::read_from(&mut self.socket).wrap_err("failed to read greetd response")
    }

    pub fn create_session(&mut self, username: String) -> Result<Response> {
        self.send_request(Request::CreateSession { username })
    }

    pub fn post_auth_message_response(&mut self, response: Option<String>) -> Result<Response> {
        self.send_request(Request::PostAuthMessageResponse { response })
    }

    pub fn start_session(&mut self, cmd: Vec<String>, env: Vec<String>) -> Result<Response> {
        self.send_request(Request::StartSession { cmd, env })
    }

    pub fn cancel_session(&mut self) -> Result<Response> {
        self.send_request(Request::CancelSession)
    }
}
