use crate::{application::*, Error};
use api_boundary as json;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use thiserror::Error;

impl From<InvalidEmailAddress> for json::Error {
    fn from(_: InvalidEmailAddress) -> Self {
        Self {
            message: "Invalid email address".to_string(),
        }
    }
}

impl From<InvalidPassword> for json::Error {
    fn from(err: InvalidPassword) -> Self {
        let InvalidPassword::TooShort(min_len) = err;
        Self {
            message: format!("Invalid password (min. length = {min_len})"),
        }
    }
}

impl From<CreateUserError> for json::Error {
    fn from(err: CreateUserError) -> Self {
        let message = match err {
            CreateUserError::UserExists => "User already exits".to_string(),
        };
        Self { message }
    }
}

impl From<LoginError> for json::Error {
    fn from(err: LoginError) -> Self {
        let message = match err {
            LoginError::InvalidEmailOrPassword => {
                "Invalid email or password".to_string()
            }
        };
        Self { message }
    }
}

impl From<LogoutError> for json::Error {
    fn from(err: LogoutError) -> Self {
        let message = match err {
            LogoutError::NotLoggedIn => "No user is logged in".to_string(),
        };
        Self { message }
    }
}

impl From<AuthError> for json::Error {
    fn from(err: AuthError) -> Self {
        let message = match err {
            AuthError::NotAuthorized => "Not authorized".to_string(),
        };
        Self { message }
    }
}

impl From<CredentialParsingError> for json::Error {
    fn from(err: CredentialParsingError) -> Self {
        match err {
            CredentialParsingError::EmailAddress(err) => err.into(),
            CredentialParsingError::Password(err) => err.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum CredentialParsingError {
    #[error(transparent)]
    EmailAddress(#[from] InvalidEmailAddress),
    #[error(transparent)]
    Password(#[from] InvalidPassword),
}

impl TryFrom<json::Credentials> for Credentials {
    type Error = CredentialParsingError;
    fn try_from(
        json::Credentials { email, password }: json::Credentials,
    ) -> Result<Self, Self::Error> {
        let email: EmailAddress = email.parse()?;
        let password = Password::try_from(password)?;
        Ok(Self { email, password })
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (code, value) = match self {
            Self::Logout(err) => {
                (StatusCode::BAD_REQUEST, json::Error::from(err))
            }
            Self::Login(err) => {
                (StatusCode::BAD_REQUEST, json::Error::from(err))
            }
            Self::Credentials(err) => {
                (StatusCode::BAD_REQUEST, json::Error::from(err))
            }
            Self::CreateUser(err) => {
                (StatusCode::BAD_REQUEST, json::Error::from(err))
            }
            Self::Auth(err) => {
                (StatusCode::UNAUTHORIZED, json::Error::from(err))
            }
        };
        (code, Json(value)).into_response()
    }
}
