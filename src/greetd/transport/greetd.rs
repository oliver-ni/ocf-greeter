use std::os::unix::net::UnixStream;

use color_eyre::eyre::{Context, Result};
use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{Request, Response};

use super::Transport;

#[derive(Debug)]
pub struct GreetdTransport(UnixStream);

impl Transport for GreetdTransport {
    fn new() -> Result<Self> {
        let socket_path = std::env::var("GREETD_SOCK").wrap_err("failed to read GREETD_SOCK")?;
        let socket = UnixStream::connect(socket_path).wrap_err("failed to connect to greetd")?;
        Ok(Self(socket))
    }

    fn send_request(&mut self, request: Request) -> Result<Response> {
        request.write_to(&mut self.0).wrap_err("failed to write to greetd")?;
        Response::read_from(&mut self.0).wrap_err("failed to read from greetd")
    }
}
