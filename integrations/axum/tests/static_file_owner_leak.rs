//! Regression tests for issue #4967

#![cfg(all(feature = "default", not(feature = "wasm")))]

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{HeaderName, HeaderValue, Request, StatusCode},
    };
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::{
        LeptosRoutes, ResponseOptions, file_and_error_handler,
        generate_route_list,
    };
    use leptos_router::{
        SsrMode,
        components::{Route, Router as LeptosRouter, Routes},
        path,
    };
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };
    use tempfile::TempDir;
    use tokio::sync::Notify;
    use tower::ServiceExt;

    const PAGE_HEADER: &str = "x-page-nonce";

    // Holds the page's resource pending, so the page render is in flight
    // while the static file is served.
    static GATE: Notify = Notify::const_new();
    static SHELL_RAN: AtomicBool = AtomicBool::new(false);

    #[component]
    fn Gated() -> impl IntoView {
        let gate = Resource::new(|| (), |_| async { GATE.notified().await });
        view! {
            <Suspense>{move || Suspend::new(async move { gate.await })}</Suspense>
        }
    }

    #[component]
    fn App() -> impl IntoView {
        view! {
            <LeptosRouter>
                <Routes fallback=|| "Not Found">
                    <Route path=path!("/page") ssr=SsrMode::Async view=Gated/>
                </Routes>
            </LeptosRouter>
        }
    }

    fn shell(_options: LeptosOptions) -> impl IntoView {
        if let Some(res) = use_context::<ResponseOptions>() {
            res.insert_header(
                HeaderName::from_static(PAGE_HEADER),
                HeaderValue::from_static("abc123"),
            );
            res.set_status(StatusCode::UNAUTHORIZED);
        }
        SHELL_RAN.store(true, Ordering::SeqCst);
        view! { <App/> }
    }

    // `#[tokio::test]` uses a current-thread runtime, so the page future and
    // the static file request below are polled on the same thread.
    #[tokio::test]
    async fn in_flight_page_render_does_not_leak_into_static_file() {
        let site_root = TempDir::new().unwrap();
        std::fs::write(site_root.path().join("favicon.svg"), "<svg/>").unwrap();
        let options = LeptosOptions::builder()
            .output_name("owner-leak")
            .site_root(site_root.path().to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();

        let routes = generate_route_list(App);
        let app: Router = Router::new()
            .leptos_routes(&options, routes, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .fallback(file_and_error_handler(shell))
            .with_state(options);

        // Drive the page until its shell has run
        let mut page = Box::pin(
            app.clone()
                .oneshot(Request::get("/page").body(Body::empty()).unwrap()),
        );
        while !SHELL_RAN.load(Ordering::SeqCst) {
            assert!(futures::poll!(&mut page).is_pending());
            tokio::task::yield_now().await;
        }

        let file = app
            .clone()
            .oneshot(Request::get("/favicon.svg").body(Body::empty()).unwrap())
            .await
            .unwrap();

        GATE.notify_one();
        // a render that loses its owner between polls never resumes; fail
        // rather than hang
        let page = tokio::time::timeout(Duration::from_secs(10), page)
            .await
            .expect("page render should finish once the gate opens")
            .unwrap();

        assert_eq!(
            (file.status(), file.headers().get(PAGE_HEADER)),
            (StatusCode::OK, None),
            "static file must not pick up the page's status or headers"
        );
        assert_eq!(
            (page.status(), page.headers().get(PAGE_HEADER)),
            (
                StatusCode::UNAUTHORIZED,
                Some(&HeaderValue::from_static("abc123"))
            ),
            "page must keep the status and headers its shell set"
        );
    }
}
