use std::collections::VecDeque;

use color_eyre::eyre::Result;
use greetd_ipc::{AuthMessageType, Request, Response};

use super::Transport;

static OTP_USERNAME: &str = "otp";
static NOPASS_USERNAME: &str = "nopass";
static PASSWORD: &str = "waddles";
static OTP: &str = "123456";

#[derive(Debug, Default)]
pub struct MockTransport {
    questions: VecDeque<(&'static str, &'static str)>,
}

impl MockTransport {
    fn next(&mut self) -> Response {
        match self.questions.pop_front() {
            None => Response::Success,
            Some((question, _)) => Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                auth_message: question.to_owned(),
            },
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
                self.questions.clear();
                if !username.contains(NOPASS_USERNAME) {
                    self.questions.push_back(("Password:", PASSWORD));
                }
                if username.contains(OTP_USERNAME) {
                    self.questions.push_back(("OTP:", OTP));
                }
                Ok(self.next())
            }

            PostAuthMessageResponse { response: None } => todo!("mock response none"),
            PostAuthMessageResponse { response: Some(_) } => Ok(self.next()),

            StartSession { cmd: _, env: _ } => {
                if self.questions.is_empty() {
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
