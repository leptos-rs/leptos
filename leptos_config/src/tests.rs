use crate::{env_w_default, from_str, Env, LeptosOptions};
use std::{net::SocketAddr, str::FromStr};

#[test]
fn from_str_env() {
    assert!(matches!(from_str("dev").unwrap(), Env::DEV));
    assert!(matches!(from_str("development").unwrap(), Env::DEV));
    assert!(matches!(from_str("DEV").unwrap(), Env::DEV));
    assert!(matches!(from_str("DEVELOPMENT").unwrap(), Env::DEV));
    assert!(matches!(from_str("prod").unwrap(), Env::PROD));
    assert!(matches!(from_str("production").unwrap(), Env::PROD));
    assert!(matches!(from_str("PROD").unwrap(), Env::PROD));
    assert!(matches!(from_str("PRODUCTION").unwrap(), Env::PROD));
    assert!(from_str("TEST").is_err());
    assert!(from_str("?").is_err());
}

#[test]
fn env_w_default_test() {
    std::env::set_var("LEPTOS_CONFIG_ENV_TEST", "custom");
    assert_eq!(
        env_w_default("LEPTOS_CONFIG_ENV_TEST", "default").unwrap(),
        String::from("custom")
    );
    std::env::remove_var("LEPTOS_CONFIG_ENV_TEST");
    assert_eq!(
        env_w_default("LEPTOS_CONFIG_ENV_TEST", "default").unwrap(),
        String::from("default")
    );
}

#[test]
fn try_from_env_test() {
    // Test config values from environment variables
    std::env::set_var("LEPTOS_OUTPUT_NAME", "app_test");
    std::env::set_var("LEPTOS_SITE_ROOT", "my_target/site");
    std::env::set_var("LEPTOS_SITE_PKG_DIR", "my_pkg");
    std::env::set_var("LEPTOS_SITE_ADDR", "0.0.0.0:80");
    std::env::set_var("LEPTOS_RELOAD_PORT", "8080");

    let config = LeptosOptions::try_from_env().unwrap();
    assert_eq!(config.output_name, "app_test");

    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
}
