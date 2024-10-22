mod greetd;
mod mock;

use color_eyre::eyre::Result;
pub use greetd::GreetdTransport;
use greetd_ipc::{Request, Response};
pub use mock::MockTransport;

pub trait Transport {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn send_request(&mut self, request: Request) -> Result<Response>;

    fn create_session(&mut self, username: String) -> Result<Response> {
        self.send_request(Request::CreateSession { username })
    }

    fn post_auth_message_response(&mut self, response: Option<String>) -> Result<Response> {
        self.send_request(Request::PostAuthMessageResponse { response })
    }

    fn start_session(&mut self, cmd: Vec<String>, env: Vec<String>) -> Result<Response> {
        self.send_request(Request::StartSession { cmd, env })
    }

    fn cancel_session(&mut self) -> Result<Response> {
        self.send_request(Request::CancelSession)
    }
}
