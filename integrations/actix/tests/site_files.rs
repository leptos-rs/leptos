//! `site_pkg_files` serves the pkg dir at its URL, and `site_root_files`
//! serves the site root at `site_root_url` without the pkg dir in it.

#![cfg(feature = "default")]

use actix_web::{App, test};
use leptos::config::LeptosOptions;
use leptos_actix::{site_pkg_files, site_root_files};
use std::path::{Path, PathBuf};

/// A new temp dir with `site/robots.txt`, `site/pkg/app.js` and
/// `pkg/app.js`.
fn layout(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "leptos_actix_site_files_{name}_{}",
        std::process::id()
    ));
    for pkg in [dir.join("site/pkg"), dir.join("pkg")] {
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("app.js"), "app js").unwrap();
    }
    std::fs::write(dir.join("site/robots.txt"), "robots").unwrap();
    dir
}

fn site_root(dir: &Path) -> String {
    dir.join("site").to_string_lossy().to_string()
}

/// Checks whether each URL is served.
async fn check(options: LeptosOptions, cases: &[(&str, bool)]) {
    let app = test::init_service(
        App::new()
            .service(site_pkg_files(&options))
            .service(site_root_files(&options)),
    )
    .await;
    for (uri, served) in cases {
        let req = test::TestRequest::get().uri(uri).to_request();
        let status = test::call_service(&app, req).await.status();
        assert_eq!(status.is_success(), *served, "{uri}: {status}");
    }
}

#[actix_web::test]
async fn site_root_skips_pkg_dir_at_other_url() {
    let dir = layout("hidden");
    let options = LeptosOptions::builder()
        .output_name("app")
        .site_root(site_root(&dir))
        .site_pkg_url("assets")
        .build()
        .unwrap();
    check(
        options,
        &[
            ("/assets/app.js", true),
            ("/robots.txt", true),
            ("/pkg", false),
            ("/pkg/app.js", false),
            ("/pkg//app.js", false),
            ("//pkg/app.js", false),
            ("/./pkg/app.js", false),
            ("/%70kg/app.js", false),
        ],
    )
    .await;
    std::fs::remove_dir_all(dir).unwrap();
}

#[actix_web::test]
async fn serves_site_root_and_pkg_under_site_root_url() {
    let dir = layout("root_url");
    let options = LeptosOptions::builder()
        .output_name("app")
        .site_root(site_root(&dir))
        .site_root_url("/static")
        .build()
        .unwrap();
    check(
        options,
        &[
            ("/static/pkg/app.js", true),
            ("/static/robots.txt", true),
            ("/pkg/app.js", false),
            ("/robots.txt", false),
        ],
    )
    .await;
    std::fs::remove_dir_all(dir).unwrap();
}

#[actix_web::test]
async fn serves_pkg_dir_outside_site_root() {
    let dir = layout("outside");
    let options = LeptosOptions::builder()
        .output_name("app")
        .site_root(site_root(&dir))
        .site_pkg_dir("../pkg")
        .site_pkg_url("assets")
        .build()
        .unwrap();
    check(options, &[("/assets/app.js", true), ("/robots.txt", true)]).await;
    std::fs::remove_dir_all(dir).unwrap();
}
