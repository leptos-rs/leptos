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
}
impl From<config::ConfigError> for LeptosConfigError {
    fn from(e: config::ConfigError) -> Self {
        Self::ConfigError(e.to_string())
    }
}
