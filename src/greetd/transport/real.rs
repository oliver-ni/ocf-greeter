use std::os::unix::net::UnixStream;

use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{Request, Response};
use thiserror::Error;

use super::Transport;

#[derive(Debug)]
pub struct GreetdTransport(UnixStream);

#[derive(Debug, Error)]
pub enum GreetdTransportError {
    #[error("failed to read GREETD_SOCK")]
    MissingEnvironment(#[from] std::env::VarError),

    #[error("failed to connect to greetd socket")]
    ConnectionError(#[from] std::io::Error),

    #[error("failed to communicate with greetd socket")]
    GreetdCodecError(#[from] greetd_ipc::codec::Error),
}

impl Transport for GreetdTransport {
    type Error = GreetdTransportError;

    fn new() -> Result<Self, Self::Error> {
        let socket_path = std::env::var("GREETD_SOCK")?;
        let socket = UnixStream::connect(socket_path)?;
        Ok(Self(socket))
    }

    fn send_request(&mut self, request: Request) -> Result<Response, Self::Error> {
        request.write_to(&mut self.0)?;
        Ok(Response::read_from(&mut self.0)?)
    }
}
