#![cfg(target_arch = "wasm32")]

//! With `<Router set_is_routing>`, a navigation keeps the previous page on
//! screen, and `is_routing` set, until the new page is ready; the tests run
//! every scenario through both `<Routes>` and `<FlatRoutes>`.

mod common;

use common::*;
use leptos::prelude::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router, Routes},
    path,
};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

/// A page whose resource, created in its body, waits for the "page" gate.
#[component]
fn Page() -> impl IntoView {
    let data = AsyncDerived::new(|| {
        let loaded = gate("page");
        async move {
            loaded.await;
            "loaded"
        }
    });
    view! {
        <Suspense fallback=|| view! { <p id="page-fallback">"loading"</p> }>
            <p id="page">{move || Suspend::new(async move { data.await })}</p>
        </Suspense>
    }
}

/// The same app, with `set_is_routing` (`holding`) or without it, rendering
/// its routes through `<Routes>` or `<FlatRoutes>` (`$routes`).
macro_rules! app {
    ($routes:ident, holding) => {
        view! {
            <Router set_is_routing=routing_setter()>
                <CaptureNavigate/>
                <a id="to-page" href="/page">"page"</a>
                <$routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                    <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                    <Route path=path!("page") view=Page/>
                </$routes>
            </Router>
        }
    };
    ($routes:ident, not_holding) => {
        view! {
            <Router>
                <CaptureNavigate/>
                <$routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                    <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                    <Route path=path!("page") view=Page/>
                </$routes>
            </Router>
        }
    };
}

fn nested() -> impl IntoView {
    app!(Routes, holding)
}

fn flat() -> impl IntoView {
    app!(FlatRoutes, holding)
}

fn nested_not_holding() -> impl IntoView {
    app!(Routes, not_holding)
}

fn flat_not_holding() -> impl IntoView {
    app!(FlatRoutes, not_holding)
}

async fn holds_the_previous_page_until_the_new_one_has_loaded(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;
    assert!(app.has("#home"));
    assert!(!is_routing());

    navigate("/page");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(!app.has("#page") && !app.has("#page-fallback"));
    assert!(is_routing());

    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!app.has("#home"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_route_holds_the_previous_page_until_the_new_one_has_loaded() {
    holds_the_previous_page_until_the_new_one_has_loaded(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_route_holds_the_previous_page_until_the_new_one_has_loaded() {
    holds_the_previous_page_until_the_new_one_has_loaded(|| flat().into_any())
        .await;
}

async fn a_link_updates_the_url_when_the_new_page_is_shown(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    app.click("#to-page");
    settle().await;
    assert!(app.has("#home"));
    assert_eq!(pathname(), "/", "the URL changes with the page");

    release("page");
    settle().await;
    assert!(app.has("#page"));
    assert_eq!(pathname(), "/page");
}

#[wasm_bindgen_test]
async fn nested_route_link_updates_the_url_when_the_new_page_is_shown() {
    a_link_updates_the_url_when_the_new_page_is_shown(|| nested().into_any())
        .await;
}

#[wasm_bindgen_test]
async fn flat_route_link_updates_the_url_when_the_new_page_is_shown() {
    a_link_updates_the_url_when_the_new_page_is_shown(|| flat().into_any())
        .await;
}

async fn without_set_is_routing_the_new_page_is_shown_at_once(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/page");
    settle().await;
    assert!(!app.has("#home"));
    assert!(app.has("#page-fallback"));
}

#[wasm_bindgen_test]
async fn nested_route_without_set_is_routing_is_shown_at_once() {
    without_set_is_routing_the_new_page_is_shown_at_once(|| {
        nested_not_holding().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_route_without_set_is_routing_is_shown_at_once() {
    without_set_is_routing_the_new_page_is_shown_at_once(|| {
        flat_not_holding().into_any()
    })
    .await;
}

/// Guards the harness: a disposed router must stop handling link clicks,
/// or it would steal them from the routers of later tests.
#[wasm_bindgen_test]
async fn a_disposed_router_no_longer_handles_link_clicks() {
    start_at("/");
    drop(mount(|| nested().into_any()));

    let app = mount(|| nested().into_any());
    settle().await;
    app.click("#to-page");
    settle().await;
    release("page");
    settle().await;
    assert!(app.has("#page"));
    assert_eq!(pathname(), "/page");
}
