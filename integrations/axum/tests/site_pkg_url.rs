//! `leptos_route_site_pkg_dir` and `RouterConfiguration` mount the pkg
//! directory at its configured URL.

#![cfg(all(feature = "default", not(feature = "wasm")))]

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{HeaderValue, Request, StatusCode, header::LOCATION},
        response::Response,
    };
    use http_body_util::BodyExt;
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::LeptosRoutes;
    use leptos_router::{
        StaticSegment,
        components::{FlatRoutes, Route, Router as LeptosRouter},
    };
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

    async fn get_response(app: Router, uri: &str) -> Response {
        app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn get(app: Router, uri: &str) -> (StatusCode, String) {
        let res = get_response(app, uri).await;
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
            .build();

        let (status, body) = get(app(options.clone()), "/pkg/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app(options), "/pkg/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("pkg fallback"));
    }

    #[tokio::test]
    async fn serves_pkg_from_a_custom_location_inside_the_site_root() {
        let site_root = TempDir::new().unwrap();
        let pkg = site_root.path().join("custom/pkg");
        pkg_dir_with_app_js(&pkg);
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_root(pkg.to_string_lossy().to_string())
            .site_pkg_url("assets")
            .build();

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
    async fn serves_independent_site_pkg_root_under_site_pkg_url() {
        let site_root = TempDir::new().unwrap();
        let pkg_dir = TempDir::new().unwrap();
        pkg_dir_with_app_js(pkg_dir.path());
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_root(pkg_dir.path().to_string_lossy().to_string())
            .site_pkg_url("static/app")
            .build();

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

    async fn assert_redirect(app: Router, uri: &str, location: &str) {
        let response = get_response(app, uri).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT, "{uri}");
        assert_eq!(response.headers()[LOCATION], location, "{uri}");
    }

    #[tokio::test]
    async fn pkg_directory_redirects_keep_the_mount_prefix() {
        let site_root = TempDir::new().unwrap();
        std::fs::create_dir_all(site_root.path().join("pkg/sub")).unwrap();
        for (pkg_url, prefix) in [(None, "/pkg"), (Some("asset"), "/asset")] {
            let mut options = LeptosOptions::builder()
                .output_name("app")
                .site_root(site_root.path().to_string_lossy().to_string())
                .build();
            options.site_pkg_url = pkg_url.map(Into::into);
            let configured = Router::new().leptos_route_configure(
                leptos_axum::RouterConfiguration::new()
                    .app(App)
                    .shell(conf_shell)
                    .state(options.clone()),
            );

            let uri = format!("{prefix}/sub");
            let location = format!("{uri}/");
            for router in [configured, app(options)] {
                assert_redirect(router.clone(), &uri, &location).await;
                assert_redirect(
                    router.clone(),
                    &format!("{uri}?x=1"),
                    &format!("{location}?x=1"),
                )
                .await;
                assert_redirect(
                    Router::new().nest("/parent", router),
                    &format!("/parent{uri}"),
                    &location,
                )
                .await;
            }
        }
    }

    #[tokio::test]
    async fn pkg_mount_preserves_a_redirect_from_the_app_fallback() {
        let site_root = TempDir::new().unwrap();
        std::fs::create_dir(site_root.path().join("pkg")).unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .app(App)
                .shell(|_: LeptosOptions| {
                    let response =
                        expect_context::<leptos_axum::ResponseOptions>();
                    response.set_status(StatusCode::TEMPORARY_REDIRECT);
                    response.insert_header(
                        LOCATION,
                        HeaderValue::from_static("/sub/"),
                    );
                    "app redirect"
                })
                .state(options),
        );

        assert_redirect(app, "/pkg/sub", "/sub/").await;
    }

    #[tokio::test]
    async fn site_root_directory_redirect_keeps_the_mount_prefix() {
        let site_root = TempDir::new().unwrap();
        let docs = site_root.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        std::fs::write(docs.join("index.html"), "docs index").unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .enable_fs_leptos_site_root("/static")
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        assert_redirect(app.clone(), "/static/docs", "/static/docs/").await;
        assert_eq!(
            get(app, "/static/docs/").await,
            (StatusCode::OK, "docs index".into()),
        );
    }

    #[component]
    fn RoutedApp() -> impl IntoView {
        view! {
            <LeptosRouter>
                <FlatRoutes fallback=|| "not-found view">
                    <Route path=StaticSegment("") view=|| "home view"/>
                    <Route path=StaticSegment("about") view=|| "about view"/>
                </FlatRoutes>
            </LeptosRouter>
        }
    }

    fn routed_shell(_: LeptosOptions) -> impl IntoView {
        let uri = expect_context::<axum::http::request::Parts>().uri;
        view! { <RoutedApp/> <p>{format!("Fallback URI [{uri}]")}</p> }
    }

    async fn assert_fallback_uri(router: Router, uri: &str, expected: &str) {
        let (status, body) = get(router, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
        assert!(body.contains("not-found view"), "{uri}: {body}");
        assert!(
            body.contains(&format!("Fallback URI [{expected}]")),
            "{uri}: {body}"
        );
    }

    #[tokio::test]
    async fn both_pkg_router_apis_restore_the_mount_prefix_for_the_fallback() {
        let site_root = TempDir::new().unwrap();
        std::fs::create_dir(site_root.path().join("pkg")).unwrap();
        for (pkg_url, uri) in
            [(None, "/pkg/about"), (Some("asset"), "/asset/about")]
        {
            let mut options = LeptosOptions::builder()
                .output_name("app")
                .site_root(site_root.path().to_string_lossy().to_string())
                .build();
            options.site_pkg_url = pkg_url.map(Into::into);
            let configured = Router::new().leptos_route_configure(
                leptos_axum::RouterConfiguration::new()
                    .app(RoutedApp)
                    .shell(routed_shell)
                    .state(options.clone()),
            );
            let direct = Router::new()
                .leptos_route_site_pkg_dir(&options, routed_shell)
                .with_state(options);

            for router in [configured, direct] {
                assert_fallback_uri(router.clone(), uri, uri).await;

                let nested = Router::new().nest("/parent", router);
                let with_query = format!("{uri}?view=full");
                let request = format!("/parent{with_query}");
                assert_fallback_uri(nested, &request, &with_query).await;
            }
        }
    }

    #[tokio::test]
    async fn site_root_fallback_restores_the_mount_prefix() {
        let site_root = TempDir::new().unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .enable_fs_leptos_site_root("/static")
                .app(RoutedApp)
                .shell(routed_shell)
                .state(options),
        );

        assert_fallback_uri(app, "/static/about", "/static/about").await;
    }

    #[tokio::test]
    async fn both_router_apis_serve_pkg_at_the_url_root() {
        let site_root = TempDir::new().unwrap();
        pkg_dir_with_app_js(&site_root.path().join("pkg"));
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_url("/")
            .build();
        let configured = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new()
                .app(App)
                .shell(conf_shell)
                .state(options.clone()),
        );

        for router in [app(options), configured] {
            let (status, body) = get(router.clone(), "/app.js").await;
            assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
            assert_eq!(
                get(router, "/pkg/app.js").await.0,
                StatusCode::NOT_FOUND
            );
        }
    }

    #[tokio::test]
    async fn configuration_serves_user_files_in_the_default_pkg_dir() {
        let site_root = TempDir::new().unwrap();
        let pkg = site_root.path().join("pkg");
        pkg_dir_with_app_js(&pkg);
        std::fs::write(pkg.join("readme.txt"), "user file").unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_url("assets")
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new_with_assets()
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/assets/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app.clone(), "/assets/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("configuration fallback"));
        let (status, body) = get(app, "/pkg/readme.txt").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "user file"));
    }

    #[tokio::test]
    async fn configuration_serves_pkg_outside_the_site_root() {
        let site_root = TempDir::new().unwrap();
        let pkg_dir = TempDir::new().unwrap();
        pkg_dir_with_app_js(pkg_dir.path());
        std::fs::write(site_root.path().join("robots.txt"), "robots").unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_root(pkg_dir.path().to_string_lossy().to_string())
            .site_pkg_url("asset")
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::new_with_assets()
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/asset/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        assert_eq!(
            get(app.clone(), "/pkg/app.js").await.0,
            StatusCode::NOT_FOUND
        );
        let (status, body) = get(app, "/robots.txt").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "robots"));
    }

    #[cfg(feature = "embed")]
    #[tokio::test]
    async fn configuration_serves_embedded_pkg_under_site_pkg_url() {
        use leptos_axum::rust_embed::{self, Embed};

        // Reuse the tracked service_mode/public/robots.txt fixture.
        #[derive(Clone, Copy, Embed)]
        #[folder = "tests/service_mode/public"]
        #[prefix = "/pkg/"]
        struct TestPkg;

        let options = LeptosOptions::builder()
            .output_name("app")
            .site_pkg_url("assets")
            .build();
        let app = Router::new().leptos_route_configure(
            leptos_axum::RouterConfiguration::embed(TestPkg)
                .app(App)
                .shell(conf_shell)
                .state(options),
        );

        let (status, body) = get(app.clone(), "/assets/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, include_str!("service_mode/public/robots.txt"));
        let (status, _) = get(app, "/pkg/robots.txt").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
