use mailparse::addrparse;
use pwhash::bcrypt;
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Default)]
pub struct AppState {
    users: HashMap<EmailAddress, Password>,
    tokens: HashMap<Uuid, EmailAddress>,
}

impl AppState {
    pub fn create_user(
        &mut self,
        credentials: Credentials,
    ) -> Result<(), CreateUserError> {
        let Credentials { email, password } = credentials;
        let user_exists = self.users.contains_key(&email);
        if user_exists {
            return Err(CreateUserError::UserExists);
        }
        self.users.insert(email, password);
        Ok(())
    }

    pub fn login(
        &mut self,
        email: EmailAddress,
        password: &str,
    ) -> Result<Uuid, LoginError> {
        let valid_credentials = self
            .users
            .get(&email)
            .map(|hashed_password| hashed_password.verify(password))
            .unwrap_or(false);
        if !valid_credentials {
            Err(LoginError::InvalidEmailOrPassword)
        } else {
            let token = Uuid::new_v4();
            self.tokens.insert(token, email);
            Ok(token)
        }
    }

    pub fn logout(&mut self, token: &str) -> Result<(), LogoutError> {
        let token = token
            .parse::<Uuid>()
            .map_err(|_| LogoutError::NotLoggedIn)?;
        self.tokens.remove(&token);
        Ok(())
    }

    pub fn authorize_user(
        &self,
        token: &str,
    ) -> Result<CurrentUser, AuthError> {
        token
            .parse::<Uuid>()
            .map_err(|_| AuthError::NotAuthorized)
            .and_then(|token| {
                self.tokens
                    .get(&token)
                    .cloned()
                    .map(|email| CurrentUser { email, token })
                    .ok_or(AuthError::NotAuthorized)
            })
    }
}

#[derive(Debug, Error)]
pub enum CreateUserError {
    #[error("The user already exists")]
    UserExists,
}

#[derive(Debug, Error)]
pub enum LoginError {
    #[error("Invalid email or password")]
    InvalidEmailOrPassword,
}

#[derive(Debug, Error)]
pub enum LogoutError {
    #[error("You are not logged in")]
    NotLoggedIn,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("You are not authorized")]
    NotAuthorized,
}

pub struct Credentials {
    pub email: EmailAddress,
    pub password: Password,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct EmailAddress(String);

#[derive(Debug, Error)]
#[error("The given email address is invalid")]
pub struct InvalidEmailAddress;

impl FromStr for EmailAddress {
    type Err = InvalidEmailAddress;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        addrparse(s)
            .ok()
            .and_then(|parsed| parsed.extract_single_info())
            .map(|single_info| Self(single_info.addr))
            .ok_or(InvalidEmailAddress)
    }
}

impl EmailAddress {
    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Clone)]
pub struct CurrentUser {
    pub email: EmailAddress,
    #[allow(dead_code)] // possibly a lint regression, this is used at line 65
    pub token: Uuid,
}

const MIN_PASSWORD_LEN: usize = 3;

pub struct Password(String);

impl Password {
    pub fn verify(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.0)
    }
}

#[derive(Debug, Error)]
pub enum InvalidPassword {
    #[error("Password is too short (min. length is {0})")]
    TooShort(usize),
}

impl TryFrom<String> for Password {
    type Error = InvalidPassword;
    fn try_from(p: String) -> Result<Self, Self::Error> {
        if p.len() < MIN_PASSWORD_LEN {
            return Err(InvalidPassword::TooShort(MIN_PASSWORD_LEN));
        }
        let hashed = bcrypt::hash(&p).unwrap();
        Ok(Self(hashed))
    }
}
