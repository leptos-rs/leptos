#![cfg(all(feature = "default", not(feature = "wasm")))]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use leptos::{config::LeptosOptions, prelude::*};
use leptos_axum::{LeptosRoutes, generate_route_list_with_ssg};
use leptos_meta::{MetaTags, provide_meta_context};
use leptos_router::{
    SsrMode,
    components::{Route, Router as LeptosRouter, Routes},
    hooks::use_params_map,
    path,
    static_routes::StaticRoute,
};
use std::sync::{
    LazyLock,
    atomic::{AtomicU64, Ordering},
};
use tokio::time::{Duration, sleep};
use tower::ServiceExt;

static TICKS: LazyLock<tokio::sync::broadcast::Sender<()>> =
    LazyLock::new(|| tokio::sync::broadcast::channel(64).0);
static RENDERS: AtomicU64 = AtomicU64::new(0);

#[component]
fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <LeptosRouter>
            <Routes fallback=|| "Not Found">
                <Route
                    path=path!("/isr/:id")
                    ssr=SsrMode::Static(StaticRoute::new().regenerate(|_| {
                        futures::stream::unfold(
                            TICKS.subscribe(),
                            |mut rx| async move {
                                rx.recv().await.ok().map(|()| ((), rx))
                            },
                        )
                    }))
                    view=|| {
                        RENDERS.fetch_add(1, Ordering::SeqCst);
                        let id = use_params_map()
                            .get_untracked().get("id").unwrap_or_default();
                        if id.starts_with('x') {
                            use_context::<leptos_axum::ResponseOptions>()
                                .unwrap()
                                .set_status(StatusCode::NOT_FOUND);
                        }
                        view! { <h1>{id}</h1> }
                    }
                />
            </Routes>
        </LeptosRouter>
    }
}

fn shell(_options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head><MetaTags/></head>
            <body><App/></body>
        </html>
    }
}

async fn wait_until(condition: impl Fn() -> bool) {
    for _ in 0..200 {
        if condition() {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(condition(), "timed out waiting for regeneration");
}

// On-demand builds must share one regeneration loop per cached path;
// uncached 404 pages must never start one, even on repeated requests.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn static_regeneration_has_one_loop_per_cached_path() {
    let site_root = std::env::temp_dir().join(format!(
        "leptos_axum_static_regeneration_loops_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&site_root).unwrap();
    let options = LeptosOptions::builder()
        .output_name("static-regeneration-loops")
        .site_root(site_root.to_string_lossy().to_string())
        .site_pkg_dir("pkg")
        .build();
    // Leave the file missing so the first requests exercise on-demand builds.
    let (routes, _generator) = generate_route_list_with_ssg(App);
    let app: Router = Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .with_state(options);
    let request = |uri: String| {
        app.clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
    };
    let responses = futures::future::join_all(
        (0..16).map(|_| request("/isr/a".to_owned())),
    )
    .await;
    for response in responses {
        assert_eq!(response.unwrap().status(), StatusCode::OK);
    }
    // Subscription happens after the initial response is sent.
    wait_until(|| TICKS.receiver_count() >= 1).await;
    sleep(Duration::from_millis(200)).await;
    assert_eq!(TICKS.receiver_count(), 1);

    // Uncached 404s must not start loops, even when requested repeatedly.
    for id in (0..20).chain(std::iter::repeat_n(0, 5)) {
        let uri = format!("/isr/x{id}");
        assert_eq!(request(uri).await.unwrap().status(), StatusCode::NOT_FOUND);
    }
    sleep(Duration::from_millis(200)).await;
    assert_eq!(TICKS.receiver_count(), 1);

    // The single loop must still regenerate the page when invalidated.
    let before = RENDERS.load(Ordering::SeqCst);
    TICKS.send(()).unwrap();
    wait_until(|| RENDERS.load(Ordering::SeqCst) > before).await;
    sleep(Duration::from_millis(200)).await;
    assert_eq!(RENDERS.load(Ordering::SeqCst), before + 1);
    std::fs::remove_dir_all(site_root).unwrap();
}
