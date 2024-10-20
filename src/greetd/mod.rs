pub mod client;
pub mod state;
pub mod transport;

use client::Client;
use state::{Empty, NeedAuthResponse, SessionCreated, SessionStarted};
use transport::Transport;

pub enum AnyClient<T: Transport> {
    Empty(Client<Empty, T>),
    NeedAuthResponse(Client<NeedAuthResponse, T>),
    SessionCreated(Client<SessionCreated, T>),
    SessionStarted(Client<SessionStarted, T>),
}

macro_rules! impl_from {
    ($state:ident) => {
        impl<T: Transport> From<Client<$state, T>> for AnyClient<T> {
            fn from(value: Client<$state, T>) -> Self {
                Self::$state(value)
            }
        }
    };
}

impl_from!(Empty);
impl_from!(NeedAuthResponse);
impl_from!(SessionCreated);
impl_from!(SessionStarted);
