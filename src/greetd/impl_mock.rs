use color_eyre::eyre::{OptionExt, Result};

use super::state::{Empty, NeedAuthResponse, SessionCreated, SessionStarted};
use super::{
    AnyClient, EmptyClient, NeedAuthResponseClient, SessionCreatedClient, SessionStartedClient,
};

pub struct MockClient<State>(State);

impl MockClient<Empty> {
    pub fn new() -> Result<Self> {
        Ok(Self(Empty))
    }
}

impl EmptyClient for MockClient<Empty> {
    fn create_session(self, _username: String) -> Result<AnyClient> {
        Ok(AnyClient::need_auth_response(MockClient(
            NeedAuthResponse {
                auth_message_type: greetd_ipc::AuthMessageType::Secret,
                auth_message: "Password".to_owned(),
            },
        )))
    }

    fn state(&self) -> &Empty {
        &self.0
    }
}

impl NeedAuthResponseClient for MockClient<NeedAuthResponse> {
    fn post_auth_message_response(self, response: Option<String>) -> Result<AnyClient> {
        let _ = response.ok_or_eyre("Missing password")?;
        Ok(AnyClient::session_created(MockClient(SessionCreated)))
    }

    fn state(&self) -> &NeedAuthResponse {
        &self.0
    }
}

impl SessionCreatedClient for MockClient<SessionCreated> {
    fn start_session(self, _cmd: Vec<String>, _env: Vec<String>) -> Result<AnyClient> {
        Ok(AnyClient::session_started(MockClient(SessionStarted)))
    }

    fn cancel_session(self) -> Result<AnyClient> {
        Ok(AnyClient::empty(MockClient(Empty)))
    }

    fn state(&self) -> &SessionCreated {
        &self.0
    }
}

impl SessionStartedClient for MockClient<SessionStarted> {
    fn state(&self) -> &SessionStarted {
        &self.0
    }
}
