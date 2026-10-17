//! `leptos_route_site_pkg_dir` and `RouterConfiguration` serve the pkg assets
//! from the `site_pkg_dir` directory at the pkg URL, and the site root at
//! `site_root_url`.

#![cfg(all(feature = "default", not(feature = "wasm")))]

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::LeptosRoutes;
    use std::path::Path;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn shell(_options: LeptosOptions) -> impl IntoView {
        view! { <p>"pkg fallback"</p> }
    }

    fn app(options: LeptosOptions) -> Router {
        // rendering the fallback page needs the executor `leptos_routes` would
        // otherwise set up
        _ = any_spawner::Executor::init_tokio();
        Router::new()
            .leptos_route_site_pkg_dir(&options, shell)
            .with_state(options)
    }

    fn pkg_dir_with_app_js(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("app.js"), "app js").unwrap();
    }

    /// A `../pkg` dir outside the site root, served at `/assets`.
    fn outside_pkg_options(dir: &TempDir) -> LeptosOptions {
        let site_root = dir.path().join("site");
        std::fs::create_dir_all(&site_root).unwrap();
        pkg_dir_with_app_js(&dir.path().join("pkg"));
        LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.to_string_lossy().to_string())
            .site_pkg_dir("../pkg")
            .site_pkg_url("assets")
            .build()
            .unwrap()
    }

    async fn get(app: Router, uri: &str) -> (StatusCode, String) {
        let res = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn serves_site_pkg_dir_under_its_own_path_by_default() {
        let site_root = TempDir::new().unwrap();
        pkg_dir_with_app_js(&site_root.path().join("pkg"));
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build()
            .unwrap();

        let (status, body) = get(app(options.clone()), "/pkg/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app(options), "/pkg/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("pkg fallback"));
    }

    #[tokio::test]
    async fn serves_site_pkg_dir_under_site_pkg_url() {
        let dir = TempDir::new().unwrap();
        let options = outside_pkg_options(&dir);

        let (status, body) = get(app(options.clone()), "/assets/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) =
            get(app(options.clone()), "/assets/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("pkg fallback"));
        // the on-disk path is no longer routed
        let (status, _) = get(app(options), "/pkg/app.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn serves_absolute_site_pkg_dir_under_site_pkg_url() {
        let site_root = TempDir::new().unwrap();
        let pkg_dir = TempDir::new().unwrap();
        pkg_dir_with_app_js(pkg_dir.path());
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir(pkg_dir.path().to_string_lossy().to_string())
            .site_pkg_url("static/app")
            .build()
            .unwrap();

        let (status, body) = get(app(options), "/static/app/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
    }

    #[component]
    fn App() -> impl IntoView {
        "app"
    }

    fn conf_shell(_: LeptosOptions) -> impl IntoView {
        view! { <p>"configuration fallback"</p> }
    }

    #[tokio::test]
    async fn configuration_serves_relative_pkg_under_site_pkg_url() {
        let dir = TempDir::new().unwrap();
        let options = outside_pkg_options(&dir);
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/assets/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app.clone(), "/assets/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("configuration fallback"));
        let (status, _) = get(app, "/pkg/app.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn configuration_serves_absolute_pkg_under_site_pkg_url() {
        let site_root = TempDir::new().unwrap();
        let pkg_dir = TempDir::new().unwrap();
        pkg_dir_with_app_js(pkg_dir.path());
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir(pkg_dir.path().to_string_lossy().to_string())
            .site_pkg_url("static/app")
            .build()
            .unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app, "/static/app/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
    }

    #[tokio::test]
    async fn configuration_serves_site_root_and_pkg_under_site_root_url() {
        let site_root = TempDir::new().unwrap();
        pkg_dir_with_app_js(&site_root.path().join("pkg"));
        std::fs::write(site_root.path().join("robots.txt"), "robots").unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_root_url("/static")
            .build()
            .unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new_with_assets()
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/static/pkg/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app.clone(), "/static/robots.txt").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "robots"));
        for path in ["/pkg/app.js", "/robots.txt"] {
            let (status, _) = get(app.clone(), path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn configuration_site_root_skips_pkg_dir_at_other_url() {
        for root_url in ["/", "/static"] {
            let site_root = TempDir::new().unwrap();
            pkg_dir_with_app_js(&site_root.path().join("pkg"));
            std::fs::write(site_root.path().join("robots.txt"), "robots")
                .unwrap();
            let options = LeptosOptions::builder()
                .output_name("app")
                .site_root(site_root.path().to_string_lossy().to_string())
                .site_root_url(root_url)
                .site_pkg_url("assets")
                .build()
                .unwrap();
            let app = Router::new().leptos_route_configure(
                leptos_axum::RouterConfiguration::new_with_assets()
                    .app(App)
                    .shell(conf_shell)
                    .state(options),
            );
            let root = root_url.trim_end_matches('/');

            let (status, body) = get(app.clone(), "/assets/app.js").await;
            assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
            let robots = format!("{root}/robots.txt");
            let (status, body) = get(app.clone(), &robots).await;
            assert_eq!((status, body.as_str()), (StatusCode::OK, "robots"));
            for path in
                ["/pkg", "/pkg/app.js", "/./pkg/app.js", "/%70kg/app.js"]
            {
                let path = format!("{root}{path}");
                let (status, body) = get(app.clone(), &path).await;
                assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
                assert!(body.contains("configuration fallback"), "{path}");
            }
        }
    }

    // Reuse the tracked service_mode/public/robots.txt fixture.
    #[cfg(feature = "embed")]
    #[derive(Clone, Copy, leptos_axum::rust_embed::Embed)]
    #[folder = "tests/service_mode/public"]
    #[prefix = "/pkg/"]
    struct TestPkg;
    #[cfg(feature = "embed")]
    use leptos_axum::rust_embed;

    #[cfg(feature = "embed")]
    #[tokio::test]
    async fn configuration_serves_embedded_pkg_under_site_root_url() {
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root_url("/static")
            .build()
            .unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::embed(TestPkg)
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/static/pkg/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, include_str!("service_mode/public/robots.txt"));
        let (status, _) = get(app, "/pkg/robots.txt").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[cfg(feature = "embed")]
    #[tokio::test]
    async fn configuration_does_not_embed_pkg_outside_site_root() {
        let dir = TempDir::new().unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::embed(TestPkg)
                .app(App)
                .shell(conf_shell)
                .state(outside_pkg_options(&dir)),
        );

        let (status, _) = get(app, "/assets/robots.txt").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // The same fixture, embedded as a pkg dir on its own.
    #[cfg(feature = "embed")]
    #[derive(Clone, Copy, leptos_axum::rust_embed::Embed)]
    #[folder = "tests/service_mode/public"]
    #[prefix = "/"]
    struct TestPkgDir;

    #[cfg(feature = "embed")]
    #[tokio::test]
    async fn configuration_serves_pkg_embedded_on_its_own() {
        let dir = TempDir::new().unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::embed(TestPkg)
                .enable_embed_site_pkg(TestPkgDir)
                .app(App)
                .shell(conf_shell)
                .state(outside_pkg_options(&dir)),
        );

        let (status, body) = get(app.clone(), "/assets/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, include_str!("service_mode/public/robots.txt"));
        let (status, body) = get(app, "/assets/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("configuration fallback"));
    }

    #[cfg(feature = "embed")]
    #[tokio::test]
    async fn configuration_embedded_site_root_skips_pkg_dir_at_other_url() {
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_pkg_url("assets")
            .build()
            .unwrap();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::embed_with_assets(TestPkg)
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/assets/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, include_str!("service_mode/public/robots.txt"));
        for path in ["/pkg/robots.txt", "/%70kg/robots.txt"] {
            let (status, body) = get(app.clone(), path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
            assert!(body.contains("configuration fallback"), "{path}");
        }
    }
}
