use std::collections::VecDeque;

use color_eyre::eyre::Result;
use greetd_ipc::{AuthMessageType, Request, Response};

use super::Transport;

static OTP_USERNAME: &str = "otp";
static NOPASS_USERNAME: &str = "nopass";

#[derive(Debug, Default)]
pub struct MockTransport {
    auth_messages: VecDeque<Response>,
}

impl MockTransport {
    fn next(&mut self) -> Response {
        match self.auth_messages.pop_front() {
            None => Response::Success,
            Some(r) => r,
        }
    }
}

impl Transport for MockTransport {
    fn new() -> Result<Self> {
        Ok(Default::default())
    }

    fn send_request(&mut self, request: Request) -> Result<Response> {
        use Request::*;

        match request {
            CreateSession { username } => {
                self.auth_messages.clear();
                if !username.contains(NOPASS_USERNAME) {
                    self.auth_messages.push_back(Response::AuthMessage {
                        auth_message: "Password: ".to_owned(),
                        auth_message_type: AuthMessageType::Secret,
                    });
                }
                if username.contains(OTP_USERNAME) {
                    self.auth_messages.push_back(Response::AuthMessage {
                        auth_message: "OTP: ".to_owned(),
                        auth_message_type: AuthMessageType::Visible,
                    });
                }
                self.auth_messages.push_back(Response::AuthMessage {
                    auth_message: "This is a test info message!".to_owned(),
                    auth_message_type: AuthMessageType::Info,
                });
                Ok(self.next())
            }

            PostAuthMessageResponse { response: None } => todo!("mock response none"),
            PostAuthMessageResponse { response: Some(_) } => Ok(self.next()),

            StartSession { cmd, env } => {
                if self.auth_messages.is_empty() {
                    println!("Session started. cmd: {:?}, env: {:?}", cmd, env);
                    Ok(Response::Success)
                } else {
                    todo!("mock start session out of order")
                }
            }

            CancelSession => {
                std::mem::take(self);
                Ok(Response::Success)
            }
        }
    }
}
