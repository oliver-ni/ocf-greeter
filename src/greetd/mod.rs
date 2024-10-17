pub mod r#impl;
pub mod impl_mock;
pub mod state;

use color_eyre::eyre::Result;
use enum_dispatch::enum_dispatch;
use impl_mock::MockClient;
use r#impl::GreetdClient;
use state::{Empty, NeedAuthResponse, SessionCreated, SessionStarted};

#[enum_dispatch]
pub trait EmptyClient {
    fn state(&self) -> &Empty;
    fn create_session(self, username: String) -> Result<AnyClient>;
}

#[enum_dispatch]
pub trait NeedAuthResponseClient {
    fn state(&self) -> &NeedAuthResponse;
    fn post_auth_message_response(self, response: Option<String>) -> Result<AnyClient>;
}

#[enum_dispatch]
pub trait SessionCreatedClient {
    fn state(&self) -> &SessionCreated;
    fn start_session(self, cmd: Vec<String>, env: Vec<String>) -> Result<AnyClient>;
    fn cancel_session(self) -> Result<AnyClient>;
}

#[enum_dispatch]
pub trait SessionStartedClient {
    fn state(&self) -> &SessionStarted;
}

// Enums so we can choose between real/mock implementations

#[enum_dispatch(EmptyClient)]
pub enum AnyEmptyClient {
    GreetdClient(GreetdClient<Empty>),
    MockClient(MockClient<Empty>),
}

#[enum_dispatch(NeedAuthResponseClient)]
pub enum AnyNeedAuthResponseClient {
    GreetdClient(GreetdClient<NeedAuthResponse>),
    MockClient(MockClient<NeedAuthResponse>),
}

#[enum_dispatch(SessionCreatedClient)]
pub enum AnySessionCreatedClient {
    GreetdClient(GreetdClient<SessionCreated>),
    MockClient(MockClient<SessionCreated>),
}

#[enum_dispatch(SessionStartedClient)]
pub enum AnySessionStartedClient {
    GreetdClient(GreetdClient<SessionStarted>),
    MockClient(MockClient<SessionStarted>),
}

pub enum AnyClient {
    EmptyClient(AnyEmptyClient),
    NeedAuthResponseClient(AnyNeedAuthResponseClient),
    SessionCreatedClient(AnySessionCreatedClient),
    SessionStartedClient(AnySessionStartedClient),
}

impl AnyClient {
    pub fn empty<T: Into<AnyEmptyClient>>(t: T) -> AnyClient {
        Self::EmptyClient(t.into())
    }
    pub fn need_auth_response<T: Into<AnyNeedAuthResponseClient>>(t: T) -> AnyClient {
        Self::NeedAuthResponseClient(t.into())
    }
    pub fn session_created<T: Into<AnySessionCreatedClient>>(t: T) -> AnyClient {
        Self::SessionCreatedClient(t.into())
    }
    pub fn session_started<T: Into<AnySessionStartedClient>>(t: T) -> AnyClient {
        Self::SessionStartedClient(t.into())
    }
}
