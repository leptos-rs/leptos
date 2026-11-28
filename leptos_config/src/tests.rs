use crate::{
    Env, LeptosOptions, ReloadWSProtocol, env_from_str, env_w_default,
    env_wo_default, ws_from_str,
};
use std::{
    net::SocketAddr,
    path::{Ancestors, Path},
    str::FromStr,
};

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
    assert!(matches!(
        ws_from_str("auto").unwrap(),
        ReloadWSProtocol::Auto
    ));

    assert!(matches!(ws_from_str("ws").unwrap(), ReloadWSProtocol::WS));
    assert!(matches!(ws_from_str("WS").unwrap(), ReloadWSProtocol::WS));
    assert!(matches!(ws_from_str("wss").unwrap(), ReloadWSProtocol::WSS));
    assert!(matches!(ws_from_str("WSS").unwrap(), ReloadWSProtocol::WSS));
    assert!(ws_from_str("TEST").is_err());
    assert!(ws_from_str("?").is_err());
}

#[test]
fn ws_from_to_str_test() {
    assert_eq!(ws_from_str("auto").unwrap().to_string(), "auto");
    assert_eq!(ws_from_str("ws").unwrap().to_string(), "ws");
    assert_eq!(ws_from_str("WS").unwrap().to_string(), "ws");
    assert_eq!(ws_from_str("wss").unwrap().to_string(), "wss");
    assert_eq!(ws_from_str("WSS").unwrap().to_string(), "wss");
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
            ("LEPTOS_SITE_PKG_ROOT", None),
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
    assert_eq!(config.site_pkg_path(), Path::new("my_target/site/my_pkg"));
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
    assert_eq!(config.env, Env::PROD);
    assert_eq!(config.reload_ws_protocol, ReloadWSProtocol::WSS)
}

#[test]
#[allow(deprecated)]
fn leptos_options_css_file_path() {
    fn next_file_name<'a>(a: &'a mut Ancestors) -> Option<&'a str> {
        a.next()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
    }
    let options = LeptosOptions::builder().output_name("test").build();
    let path = options.css_file_path();
    let mut ancestors = path.ancestors();
    assert_eq!(next_file_name(&mut ancestors), Some("test.css"));
    assert_eq!(next_file_name(&mut ancestors), Some("pkg"));
    assert_eq!(next_file_name(&mut ancestors), None);

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("")
        .build();
    let path = options.css_file_path();
    let mut ancestors = path.ancestors();
    assert_eq!(next_file_name(&mut ancestors), Some("test.css"));
    assert_eq!(next_file_name(&mut ancestors), None);

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("my_pkg")
        .site_root("my_site")
        .build();
    let path = options.css_file_path();
    let mut ancestors = path.ancestors();
    assert_eq!(next_file_name(&mut ancestors), Some("test.css"));
    assert_eq!(next_file_name(&mut ancestors), Some("my_pkg"));
    assert_eq!(next_file_name(&mut ancestors), Some("my_site"));
    assert_eq!(next_file_name(&mut ancestors), None);
}

#[test]
#[allow(deprecated)]
fn leptos_options_css_path() {
    let options = LeptosOptions::builder().output_name("test").build();
    assert_eq!(options.css_path(), "/pkg/test.css");

    let options = LeptosOptions::builder()
        .output_name("test.css")
        .site_pkg_dir("my/pkg")
        .build();
    assert_eq!(options.css_path(), "/my/pkg/test.css.css");

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("/pkg/")
        .build();
    assert_eq!(options.css_path(), "/pkg/test.css");

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("")
        .build();
    assert_eq!(options.css_path(), "/test.css");
}

#[test]
#[allow(deprecated)]
fn leptos_options_pkg_url_path() {
    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("my/pkg")
        .build();
    assert_eq!(options.pkg_url_path(), "my/pkg");
    assert_eq!(
        options.site_pkg_dir_route_base(),
        options.site_pkg_route_base()
    );

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("/srv/site/pkg")
        .site_pkg_url("/assets/")
        .build();
    assert_eq!(options.pkg_url_path(), "assets");
    assert_eq!(options.css_path(), "/assets/test.css");
}

#[test]
fn site_pkg_root_is_relative_to_the_working_directory() {
    let options = LeptosOptions::builder()
        .output_name("test")
        .site_root("site")
        .site_pkg_root("build/assets")
        .build();

    assert_eq!(options.site_pkg_path(), Path::new("build/assets"));
    assert_eq!(options.css_file_path(), Path::new("build/assets/test.css"));
    assert_eq!(options.site_pkg_route_base(), "/pkg/");
}

#[test]
#[allow(deprecated)]
fn pkg_dir_warning_is_only_for_nondefault_legacy_values() {
    let mut options = LeptosOptions::builder().output_name("test").build();
    options.warn_on_site_pkg_dir(|warning| panic!("{warning}"));

    options.site_pkg_dir = Some("pkg".into());
    options.warn_on_site_pkg_dir(|warning| panic!("{warning}"));

    options.site_pkg_dir = Some("custom".into());
    let mut warnings = Vec::new();
    options.warn_on_site_pkg_dir(|warning| warnings.push(warning.to_owned()));
    assert_eq!(warnings.len(), 1);
    for key in ["LEPTOS_SITE_PKG_DIR", "site-pkg-root", "site-pkg-url"] {
        assert!(warnings[0].contains(key));
    }
}

#[test]
#[allow(deprecated)]
fn site_pkg_url_defaults_to_the_current_site_root_url_and_pkg_name() {
    let mut options = LeptosOptions::builder().output_name("test").build();
    assert_eq!(options.site_root_route_base(), "/");

    options.site_root_url = "/static///".into();
    assert_eq!(options.site_root_route_base(), "/static/");
    assert_eq!(options.site_pkg_route_base(), "/static/pkg/");
    assert_eq!(options.css_path(), "/static/pkg/test.css");

    options.site_pkg_dir = Some("/legacy/pkg/".into());
    assert_eq!(options.site_pkg_route_base(), "/static/legacy/pkg/");

    options.site_pkg_url = Some("/".into());
    assert_eq!(options.site_pkg_route_base(), "/");
}
