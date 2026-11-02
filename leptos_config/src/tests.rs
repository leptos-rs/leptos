use crate::{
    Env, LeptosOptions, ReloadWSProtocol, env_from_str, env_w_default,
    env_wo_default, ws_from_str,
};
use std::{
    net::SocketAddr,
    path::{Ancestors, Component, Path, PathBuf},
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
            ("LEPTOS_SITE_ROOT_URL", Some("/static")),
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
    assert_eq!(config.site_root_url.as_ref(), "/static");
    assert_eq!(config.site_pkg_dir.as_ref(), "my_pkg");
    assert_eq!(config.pkg_url_path(), "static/my_pkg");
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
fn leptos_options_css_file_path() {
    fn next_file_name<'a>(a: &'a mut Ancestors) -> Option<&'a str> {
        a.next()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
    }
    let options = LeptosOptions::builder()
        .output_name("test")
        .build()
        .unwrap();
    let path = options.css_file_path();
    let mut ancestors = path.ancestors();
    assert_eq!(next_file_name(&mut ancestors), Some("test.css"));
    assert_eq!(next_file_name(&mut ancestors), Some("pkg"));
    assert_eq!(next_file_name(&mut ancestors), None);

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("my_pkg")
        .site_root("my_site")
        .build()
        .unwrap();
    let path = options.css_file_path();
    let mut ancestors = path.ancestors();
    assert_eq!(next_file_name(&mut ancestors), Some("test.css"));
    assert_eq!(next_file_name(&mut ancestors), Some("my_pkg"));
    assert_eq!(next_file_name(&mut ancestors), Some("my_site"));
    assert_eq!(next_file_name(&mut ancestors), None);
}

#[test]
fn leptos_options_css_path() {
    let options = LeptosOptions::builder()
        .output_name("test")
        .build()
        .unwrap();
    assert_eq!(options.css_path(), "/pkg/test.css");

    let options = LeptosOptions::builder()
        .output_name("test.css")
        .site_pkg_dir("my/pkg")
        .build()
        .unwrap();
    assert_eq!(options.css_path(), "/my/pkg/test.css.css");

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_url("/pkg/")
        .build()
        .unwrap();
    assert_eq!(options.css_path(), "/pkg/test.css");
}

#[test]
fn leptos_options_pkg_url_path() {
    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("my/pkg")
        .build()
        .unwrap();
    assert_eq!(options.pkg_url_path(), "my/pkg");

    let options = LeptosOptions::builder()
        .output_name("test")
        .site_pkg_dir("/srv/site/pkg")
        .site_pkg_url("/assets/")
        .build()
        .unwrap();
    assert_eq!(options.pkg_url_path(), "assets");
    assert_eq!(options.css_path(), "/assets/test.css");
}

#[test]
fn site_layout() {
    use crate::errors::LeptosConfigError;

    fn build(
        root: &str,
        root_url: &str,
        dir: &str,
        url: Option<&str>,
    ) -> Result<LeptosOptions, LeptosConfigError> {
        let builder = LeptosOptions::builder()
            .output_name("app")
            .site_root(root)
            .site_root_url(root_url)
            .site_pkg_dir(dir);
        match url {
            Some(url) => builder.site_pkg_url(url).build(),
            None => builder.build(),
        }
    }

    // A pkg dir inside the site root, given as an absolute path while the
    // site root is relative, is compared from the current directory.
    let cwd_pkg = std::env::current_dir().unwrap().join("pkg");
    let cwd_pkg = cwd_pkg.to_str().unwrap();
    // On Windows, the same path without its drive has a root but is not
    // absolute.
    let rooted_pkg: PathBuf = Path::new(cwd_pkg)
        .components()
        .filter(|c| !matches!(c, Component::Prefix(_)))
        .collect();
    let rooted_pkg = rooted_pkg.to_str().unwrap();
    // (site root, its URL, pkg dir, pkg URL, pkg URL in use, hidden pkg dir)
    for (root, root_url, dir, url, expected, hidden) in [
        ("target/site", "/", "pkg", None, "pkg", None),
        ("target/site", "/static/", "pkg", None, "static/pkg", None),
        ("target/site", "/", "pkg", Some("/pkg/"), "pkg", None),
        ("target/site", "/", "pkg", Some(""), "pkg", None),
        ("target/site", "/", "pkg", Some("js"), "js", Some("pkg")),
        ("site", "/static", "pkg", Some("pkg"), "pkg", Some("pkg")),
        ("target/site", "/", "./a/../pkg", None, "pkg", None),
        ("/srv/site", "/", "/srv/site/a/pkg", None, "a/pkg", None),
        ("/srv", "/", "/srv/a/pkg", Some("js"), "js", Some("a/pkg")),
        ("target/site", "/", "../pkg", Some("assets"), "assets", None),
        (".", "/", cwd_pkg, None, "pkg", None),
        (".", "/", rooted_pkg, None, "pkg", None),
    ] {
        let options = build(root, root_url, dir, url).unwrap();
        let case = format!("{root} {root_url} {dir} {url:?}");
        assert_eq!(options.pkg_url_path(), expected, "{case}");
        assert_eq!(options.hidden_site_pkg_dir().as_deref(), hidden, "{case}");
    }

    assert!(matches!(
        build("target/site", "/", "../pkg", None),
        Err(LeptosConfigError::SitePkgUrlRequired { .. })
    ));
    for (dir, url) in [("../pkg", "/static/"), ("pkg", "static")] {
        assert!(matches!(
            build("target/site", "/static", dir, Some(url)),
            Err(LeptosConfigError::SitePkgUrlIsSiteRootUrl { .. })
        ));
    }
    for dir in ["", ".", ".."] {
        assert!(matches!(
            build("target/site", "/", dir, None),
            Err(LeptosConfigError::SitePkgDirContainsSiteRoot { .. })
        ));
    }
}
