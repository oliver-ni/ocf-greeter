#[derive(Debug, Clone, Copy)]
pub enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

impl From<greetd_ipc::AuthMessageType> for AuthMessageType {
    fn from(value: greetd_ipc::AuthMessageType) -> Self {
        match value {
            greetd_ipc::AuthMessageType::Visible => Self::Visible,
            greetd_ipc::AuthMessageType::Secret => Self::Secret,
            greetd_ipc::AuthMessageType::Info => Self::Info,
            greetd_ipc::AuthMessageType::Error => Self::Error,
        }
    }
}

#[derive(Debug)]
pub struct Empty;

#[derive(Debug)]
pub struct NeedAuthResponse {
    pub auth_message_type: AuthMessageType,
    pub auth_message: String,
}

#[derive(Debug)]
pub struct SessionCreated;

#[derive(Debug)]
pub struct SessionStarted;
