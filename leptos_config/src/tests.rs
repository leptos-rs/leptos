use crate::{
    env_from_str, env_w_default, env_wo_default, ws_from_str, Env,
    LeptosOptions, ReloadWSProtocol,
};
use std::{net::SocketAddr, str::FromStr};

#[test]
fn env_from_str_test() {
    assert!(matches!(env_from_str("dev").unwrap(), Env::DEV));
    assert!(matches!(env_from_str("development").unwrap(), Env::DEV));
    assert!(matches!(env_from_str("DEV").unwrap(), Env::DEV));
    assert!(matches!(env_from_str("DEVELOPMENT").unwrap(), Env::DEV));
    assert!(matches!(env_from_str("prod").unwrap(), Env::PROD));
    assert!(matches!(env_from_str("production").unwrap(), Env::PROD));
    assert!(matches!(env_from_str("PROD").unwrap(), Env::PROD));
    assert!(matches!(env_from_str("PRODUCTION").unwrap(), Env::PROD));
    assert!(env_from_str("TEST").is_err());
    assert!(env_from_str("?").is_err());
}

#[test]
fn ws_from_str_test() {
    assert!(matches!(ws_from_str("ws").unwrap(), ReloadWSProtocol::WS));
    assert!(matches!(ws_from_str("WS").unwrap(), ReloadWSProtocol::WS));
    assert!(matches!(ws_from_str("wss").unwrap(), ReloadWSProtocol::WSS));
    assert!(matches!(ws_from_str("WSS").unwrap(), ReloadWSProtocol::WSS));
    assert!(ws_from_str("TEST").is_err());
    assert!(ws_from_str("?").is_err());
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
fn env_wo_default_test() {
    std::env::set_var("LEPTOS_CONFIG_ENV_TEST", "custom");
    assert_eq!(
        env_wo_default("LEPTOS_CONFIG_ENV_TEST").unwrap(),
        Some(String::from("custom"))
    );
    std::env::remove_var("LEPTOS_CONFIG_ENV_TEST");
    assert_eq!(env_wo_default("LEPTOS_CONFIG_ENV_TEST").unwrap(), None);
}

#[test]
fn try_from_env_test() {
    // Test config values from environment variables
    std::env::set_var("LEPTOS_OUTPUT_NAME", "app_test");
    std::env::set_var("LEPTOS_SITE_ROOT", "my_target/site");
    std::env::set_var("LEPTOS_SITE_PKG_DIR", "my_pkg");
    std::env::set_var("LEPTOS_SITE_ADDR", "0.0.0.0:80");
    std::env::set_var("LEPTOS_RELOAD_PORT", "8080");
    std::env::set_var("LEPTOS_RELOAD_EXTERNAL_PORT", "8080");
    std::env::set_var("LEPTOS_ENV", "PROD");
    std::env::set_var("LEPTOS_RELOAD_WS_PROTOCOL", "WSS");

    let config = LeptosOptions::try_from_env().unwrap();
    assert_eq!(config.output_name, "app_test");

    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
    assert_eq!(config.env, Env::PROD);
    assert_eq!(config.reload_ws_protocol, ReloadWSProtocol::WSS)
}
