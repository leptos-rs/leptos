use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub email: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}
