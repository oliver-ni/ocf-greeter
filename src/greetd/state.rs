use greetd_ipc::AuthMessageType;

pub struct Empty;
pub struct NeedAuthResponse {
    pub auth_message_type: AuthMessageType,
    pub auth_message: String,
}
pub struct SessionCreated;
pub struct SessionStarted;
