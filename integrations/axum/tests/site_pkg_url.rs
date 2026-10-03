//! `leptos_route_site_pkg_dir` serves the pkg assets under `site_pkg_url`
//! when it is set, from the `site_pkg_dir` directory.

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
            .build();

        let (status, body) = get(app(options.clone()), "/pkg/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
        let (status, body) = get(app(options), "/pkg/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("pkg fallback"));
    }

    #[tokio::test]
    async fn serves_site_pkg_dir_under_site_pkg_url() {
        let site_root = TempDir::new().unwrap();
        pkg_dir_with_app_js(&site_root.path().join("pkg"));
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir("pkg")
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
    async fn serves_absolute_site_pkg_dir_under_site_pkg_url() {
        let site_root = TempDir::new().unwrap();
        let pkg_dir = TempDir::new().unwrap();
        pkg_dir_with_app_js(pkg_dir.path());
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir(pkg_dir.path().to_string_lossy().to_string())
            .site_pkg_url("static/app")
            .build();

        let (status, body) = get(app(options), "/static/app/app.js").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "app js"));
    }
}
