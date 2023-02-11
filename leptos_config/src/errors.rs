use std::{net::AddrParseError, num::ParseIntError};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum LeptosConfigError {
    #[error("Cargo.toml not found in package root")]
    ConfigNotFound,
    #[error("package.metadata.leptos section missing from Cargo.toml")]
    ConfigSectionNotFound,
    #[error("Failed to get Leptos Environment. Did you set LEPTOS_ENV?")]
    EnvError,
    #[error("Config Error: {0}")]
    ConfigError(String),
    #[error("Config Error: {0}")]
    EnvVarError(String),
}
impl From<config::ConfigError> for LeptosConfigError {
    fn from(e: config::ConfigError) -> Self {
        Self::ConfigError(e.to_string())
    }
}

impl From<ParseIntError> for LeptosConfigError {
    fn from(e: ParseIntError) -> Self {
        Self::ConfigError(e.to_string())
    }
}

impl From<AddrParseError> for LeptosConfigError {
    fn from(e: AddrParseError) -> Self {
        Self::ConfigError(e.to_string())
    }
}
