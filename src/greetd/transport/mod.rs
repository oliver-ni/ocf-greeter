mod mock;
mod real;

use greetd_ipc::{Request, Response};
pub use mock::MockTransport;
pub use real::GreetdTransport;

pub trait Transport {
    type Error: Send + Sync + std::error::Error + 'static;

    fn new() -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn send_request(&mut self, request: Request) -> Result<Response, Self::Error>;
}
