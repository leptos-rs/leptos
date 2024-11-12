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
    temp_env::with_var("LEPTOS_CONFIG_ENV_TEST", Some("custom"), || {
        assert_eq!(
            env_w_default("LEPTOS_CONFIG_ENV_TEST", "default").unwrap(),
            String::from("custom")
        );
    });

    temp_env::with_var_unset("LEPTOS_CONFIG_ENV_TEST", || {
        assert_eq!(
            env_w_default("LEPTOS_CONFIG_ENV_TEST", "default").unwrap(),
            String::from("default")
        );
    });
}

#[test]
fn env_wo_default_test() {
    temp_env::with_var("LEPTOS_CONFIG_ENV_TEST", Some("custom"), || {
        assert_eq!(
            env_wo_default("LEPTOS_CONFIG_ENV_TEST").unwrap(),
            Some(String::from("custom"))
        );
    });

    temp_env::with_var_unset("LEPTOS_CONFIG_ENV_TEST", || {
        assert_eq!(env_wo_default("LEPTOS_CONFIG_ENV_TEST").unwrap(), None);
    });
}

#[test]
fn try_from_env_test() {
    // Test config values from environment variables
    let config = temp_env::with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", Some("app_test")),
            ("LEPTOS_SITE_ROOT", Some("my_target/site")),
            ("LEPTOS_SITE_PKG_DIR", Some("my_pkg")),
            ("LEPTOS_SITE_ADDR", Some("0.0.0.0:80")),
            ("LEPTOS_RELOAD_PORT", Some("8080")),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", Some("8080")),
            ("LEPTOS_ENV", Some("PROD")),
            ("LEPTOS_RELOAD_WS_PROTOCOL", Some("WSS")),
        ],
        || LeptosOptions::try_from_env().unwrap(),
    );

    assert_eq!(config.output_name.as_ref(), "app_test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
    assert_eq!(config.env, Env::PROD);
    assert_eq!(config.reload_ws_protocol, ReloadWSProtocol::WSS)
}
