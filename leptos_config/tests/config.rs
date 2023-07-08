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
env = "PROD"
"#;

const CARGO_TOML_CONTENT_ERR: &str = r#"\
[package.metadata.leptos]
_output-name = "app-test"
_site-root = "my_target/site"
_site-pkg-dir = "my_pkg"
_site-addr = "0.0.0.0:80"
_reload-port = "8080"
_env = "PROD"
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

    let config = get_configuration(Some(&path_s))
        .await
        .unwrap()
        .leptos_options;

    assert_eq!(config.output_name, "app-test");
    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
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
    assert!(get_configuration(Some(&path_s)).await.is_err());
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
    assert!(get_configuration(Some(&path_s)).await.is_err());
}

#[tokio::test]
async fn get_config_from_file_ok() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_OK}").unwrap();
    }

    let config = get_config_from_file(&cargo_tmp)
        .await
        .unwrap()
        .leptos_options;

    assert_eq!(config.output_name, "app-test");
    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
}

#[tokio::test]
async fn get_config_from_file_invalid() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "{CARGO_TOML_CONTENT_ERR}").unwrap();
    }
    assert!(get_config_from_file(&cargo_tmp).await.is_err());
}

#[tokio::test]
async fn get_config_from_file_empty() {
    let cargo_tmp = NamedTempFile::new().unwrap();
    {
        let mut output = File::create(&cargo_tmp).unwrap();
        write!(output, "").unwrap();
    }
    assert!(get_config_from_file(&cargo_tmp).await.is_err());
}

#[test]
fn get_config_from_str_content() {
    let config = get_config_from_str(CARGO_TOML_CONTENT_OK)
        .unwrap()
        .leptos_options;
    assert_eq!(config.output_name, "app-test");
    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);
}

#[tokio::test]
async fn get_config_from_env() {
    // Test config values from environment variables
    std::env::set_var("LEPTOS_OUTPUT_NAME", "app-test");
    std::env::set_var("LEPTOS_SITE_ROOT", "my_target/site");
    std::env::set_var("LEPTOS_SITE_PKG_DIR", "my_pkg");
    std::env::set_var("LEPTOS_SITE_ADDR", "0.0.0.0:80");
    std::env::set_var("LEPTOS_RELOAD_PORT", "8080");

    let config = get_configuration(None).await.unwrap().leptos_options;
    assert_eq!(config.output_name, "app-test");

    assert_eq!(config.site_root, "my_target/site");
    assert_eq!(config.site_pkg_dir, "my_pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("0.0.0.0:80").unwrap()
    );
    assert_eq!(config.reload_port, 8080);

    // Test default config values
    std::env::remove_var("LEPTOS_SITE_ROOT");
    std::env::remove_var("LEPTOS_SITE_PKG_DIR");
    std::env::remove_var("LEPTOS_SITE_ADDR");
    std::env::remove_var("LEPTOS_RELOAD_PORT");

    let config = get_configuration(None).await.unwrap().leptos_options;
    assert_eq!(config.site_root, "target/site");
    assert_eq!(config.site_pkg_dir, "pkg");
    assert_eq!(
        config.site_addr,
        SocketAddr::from_str("127.0.0.1:3000").unwrap()
    );
    assert_eq!(config.reload_port, 3001);
}

#[test]
fn leptos_options_builder_default() {
    let conf = LeptosOptions::builder().output_name("app-test").build();
    assert_eq!(conf.output_name, "app-test");
    assert!(matches!(conf.env, Env::DEV));
    assert_eq!(conf.site_pkg_dir, "pkg");
    assert_eq!(conf.site_root, ".");
    assert_eq!(
        conf.site_addr,
        SocketAddr::from_str("127.0.0.1:3000").unwrap()
    );
    assert_eq!(conf.reload_port, 3001);
}
