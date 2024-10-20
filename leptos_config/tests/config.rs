use leptos_config::{
    get_config_from_file, get_config_from_str, get_configuration, Env,
    LeptosOptions,
};
use std::{fs::File, io::Write, net::SocketAddr, path::Path, str::FromStr};
use tempfile::NamedTempFile;

#[test]
fn env_default() {
    assert!(matches!(Env::default(), Env::DEV));
}

const CARGO_TOML_CONTENT_OK: &str = r#"\
[package.metadata.leptos]
output-name = "app-test"
site-root = "my_target/site"
site-pkg-dir = "my_pkg"
site-addr = "0.0.0.0:80"
reload-port = "8080"
reload-external-port = "8080"
env = "PROD"
"#;

const CARGO_TOML_CONTENT_ERR: &str = r#"\
[package.metadata.leptos]
- invalid toml -
"#;

#[tokio::test]
async fn get_configuration_from_file_ok() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_OK}").unwrap();
    }

    let path: &Path = cargo_tmp.as_ref();
    let path_s = path.to_string_lossy().to_string();

    let config = temp_env::async_with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", None::<&str>),
            ("LEPTOS_SITE_ROOT", None::<&str>),
            ("LEPTOS_SITE_PKG_DIR", None::<&str>),
            ("LEPTOS_SITE_ADDR", None::<&str>),
            ("LEPTOS_RELOAD_PORT", None::<&str>),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", None::<&str>),
        ],
        async { get_configuration(Some(&path_s)).unwrap().leptos_options },
    )
    .await;

    assert_eq!(config.output_name.as_ref(), "app-test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
}

#[tokio::test]
async fn get_configuration_from_invalid_file() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_ERR}").unwrap();
    }
    let path: &Path = cargo_tmp.as_ref();
    let path_s = path.to_string_lossy().to_string();
    assert!(get_configuration(Some(&path_s)).is_err());
}

#[tokio::test]
async fn get_configuration_from_empty_file() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "").unwrap();
    }
    let path: &Path = cargo_tmp.as_ref();
    let path_s = path.to_string_lossy().to_string();
    assert!(get_configuration(Some(&path_s)).is_err());
}

#[tokio::test]
async fn get_config_from_file_ok() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_OK}").unwrap();
    }

    let config = temp_env::async_with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", None::<&str>),
            ("LEPTOS_SITE_ROOT", None::<&str>),
            ("LEPTOS_SITE_PKG_DIR", None::<&str>),
            ("LEPTOS_SITE_ADDR", None::<&str>),
            ("LEPTOS_RELOAD_PORT", None::<&str>),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", None::<&str>),
        ],
        async { get_config_from_file(&cargo_tmp).unwrap().leptos_options },
    )
    .await;

    assert_eq!(config.output_name.as_ref(), "app-test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
}

#[tokio::test]
async fn get_config_from_file_invalid() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_ERR}").unwrap();
    }
    assert!(get_config_from_file(&cargo_tmp).is_err());
}

#[tokio::test]
async fn get_config_from_file_empty() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "").unwrap();
    }
    assert!(get_config_from_file(&cargo_tmp).is_err());
}

#[test]
fn get_config_from_str_content() {
    let config = temp_env::with_vars_unset(
        [
            "LEPTOS_OUTPUT_NAME",
            "LEPTOS_SITE_ROOT",
            "LEPTOS_SITE_PKG_DIR",
            "LEPTOS_SITE_ADDR",
            "LEPTOS_RELOAD_PORT",
            "LEPTOS_RELOAD_EXTERNAL_PORT",
        ],
        || get_config_from_str(CARGO_TOML_CONTENT_OK).unwrap(),
    );

    assert_eq!(config.output_name.as_ref(), "app-test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));
}

#[tokio::test]
async fn get_config_from_env() {
    // Test config values from environment variables
    let config = temp_env::async_with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", Some("app-test")),
            ("LEPTOS_SITE_ROOT", Some("my_target/site")),
            ("LEPTOS_SITE_PKG_DIR", Some("my_pkg")),
            ("LEPTOS_SITE_ADDR", Some("0.0.0.0:80")),
            ("LEPTOS_RELOAD_PORT", Some("8080")),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", Some("8080")),
        ],
        async { get_configuration(None).unwrap().leptos_options },
    )
    .await;

    assert_eq!(config.output_name.as_ref(), "app-test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));

    // Test default config values
    let config = temp_env::async_with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", None::<&str>),
            ("LEPTOS_SITE_ROOT", None::<&str>),
            ("LEPTOS_SITE_PKG_DIR", None::<&str>),
            ("LEPTOS_SITE_ADDR", None::<&str>),
            ("LEPTOS_RELOAD_PORT", None::<&str>),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", None::<&str>),
        ],
        async { get_configuration(None).unwrap().leptos_options },
    )
    .await;

    assert_eq!(config.site_root.as_ref(), "target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("127.0.0.1:3000").unwrap()
    );
    assert_eq!(config.reload_port, 3001);
    assert_eq!(config.reload_external_port, None);
}

#[test]
fn leptos_options_builder_default() {
    let conf = LeptosOptions::builder().output_name("app-test").build();
    assert_eq!(conf.output_name.as_ref(), "app-test");
    assert!(matches!(conf.env, Env::DEV));
    assert_eq!(conf.site_pkg_dir.as_ref(), "pkg");
    assert_eq!(conf.site_root.as_ref(), ".");
    assert_eq!(
        conf.site_addr,
        SocketAddr::from_str("127.0.0.1:3000").unwrap()
    );
    assert_eq!(conf.reload_port, 3001);
    assert_eq!(conf.reload_external_port, None);
}

#[test]
fn environment_variable_override() {
    // first check without variables set
    let config = temp_env::with_vars_unset(
        [
            "LEPTOS_OUTPUT_NAME",
            "LEPTOS_SITE_ROOT",
            "LEPTOS_SITE_PKG_DIR",
            "LEPTOS_SITE_ADDR",
            "LEPTOS_RELOAD_PORT",
            "LEPTOS_RELOAD_EXTERNAL_PORT",
        ],
        || get_config_from_str(CARGO_TOML_CONTENT_OK).unwrap(),
    );

    assert_eq!(config.output_name.as_ref(), "app-test");
    assert_eq!(config.site_root.as_ref(), "my_target/site");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
    assert_eq!(config.reload_external_port, Some(8080));

    // check the override
    let config = temp_env::with_vars(
        [
            ("LEPTOS_OUTPUT_NAME", Some("app-test2")),
            ("LEPTOS_SITE_ROOT", Some("my_target/site2")),
            ("LEPTOS_SITE_PKG_DIR", Some("my_pkg2")),
            ("LEPTOS_SITE_ADDR", Some("0.0.0.0:82")),
            ("LEPTOS_RELOAD_PORT", Some("8082")),
            ("LEPTOS_RELOAD_EXTERNAL_PORT", Some("8082")),
        ],
        || get_config_from_str(CARGO_TOML_CONTENT_OK).unwrap(),
    );

    assert_eq!(config.output_name.as_ref(), "app-test2");
    assert_eq!(config.site_root.as_ref(), "my_target/site2");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg2");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:82").unwrap()
    );
    assert_eq!(config.reload_port, 8082);
    assert_eq!(config.reload_external_port, Some(8082));
}
