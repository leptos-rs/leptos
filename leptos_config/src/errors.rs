use std::{net::AddrParseError, num::ParseIntError, str::ParseBoolError};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
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
    #[error(
        "site-pkg-dir `{site_pkg_dir}` must not be site-root `{site_root}` or \
         contain it"
    )]
    SitePkgDirContainsSiteRoot {
        site_pkg_dir: String,
        site_root: String,
    },
    #[error(
        "site-pkg-dir `{site_pkg_dir}` is outside site-root `{site_root}`, so \
         site-pkg-url must be set"
    )]
    SitePkgUrlRequired {
        site_pkg_dir: String,
        site_root: String,
    },
    #[error("site-pkg-url `{url}` must differ from site-root-url")]
    SitePkgUrlIsSiteRootUrl { url: String },
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

impl From<ParseBoolError> for LeptosConfigError {
    fn from(e: ParseBoolError) -> Self {
        Self::ConfigError(e.to_string())
    }
}
