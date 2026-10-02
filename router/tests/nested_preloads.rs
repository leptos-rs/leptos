#![cfg(target_arch = "wasm32")]

//! `<Routes>` preloads the view of each outlet a navigation shows, and must
//! only cancel the preloads of the outlets a later navigation replaces or
//! removes: an abandoned preload must never install its view over another
//! route, and a reused outlet's preload must still install its view. Every
//! scenario runs with and without `set_is_routing`.

mod common;

use common::*;
use leptos::prelude::*;
use leptos_router::{
    Lazy, LazyRoute,
    components::{Outlet, ParentRoute, Route, Router, Routes},
    path,
};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

/// A lazy route whose preload and view each wait for a "lazy" gate.
struct LazyPage;

impl LazyRoute for LazyPage {
    fn data() -> Self {
        LazyPage
    }

    async fn view(_this: Self) -> AnyView {
        gate("lazy").await;
        view! { <p id="lazy">"lazy"</p> }.into_any()
    }

    async fn preload() {
        gate("lazy").await;
    }
}

/// A lazy layout whose preload and view each wait for a "lazy" gate.
struct LazyLayout;

impl LazyRoute for LazyLayout {
    fn data() -> Self {
        LazyLayout
    }

    async fn view(_this: Self) -> AnyView {
        gate("lazy").await;
        view! { <p id="lazy-layout">"lazy layout"</p><Outlet/> }.into_any()
    }

    async fn preload() {
        gate("lazy").await;
    }
}

fn lazy_page() -> Lazy<LazyPage> {
    Lazy::new()
}

fn lazy_layout() -> Lazy<LazyLayout> {
    Lazy::new()
}

macro_rules! routes {
    () => {
        view! {
            <CaptureNavigate/>
            <Routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                <Route path=path!("static") view=|| view! { <p id="static">"static"</p> }/>
                <Route path=path!("lazy") view=lazy_page()/>
                <ParentRoute
                    path=path!("parent")
                    view=|| view! { <p id="parent">"parent"</p><Outlet/> }
                >
                    <Route path=path!("lazy") view=lazy_page()/>
                </ParentRoute>
                <ParentRoute path=path!("lazy-layout") view=lazy_layout()>
                    <Route path=path!("a") view=|| view! { <p id="child-a">"a"</p> }/>
                    <Route path=path!("b") view=|| view! { <p id="child-b">"b"</p> }/>
                </ParentRoute>
            </Routes>
        }
    };
}

fn holding() -> AnyView {
    view! { <Router set_is_routing=routing_setter()>{routes!()}</Router> }
        .into_any()
}

fn not_holding() -> AnyView {
    view! { <Router>{routes!()}</Router> }.into_any()
}

/// Lets a pending lazy route load: its preload, then its view.
async fn load_lazy_routes() {
    release("lazy");
    settle().await;
    release("lazy");
    settle().await;
}

async fn an_abandoned_preload_does_not_replace_the_new_route(
    app: fn() -> AnyView,
) {
    start_at("/nowhere");
    let app = mount(app);
    settle().await;
    assert!(app.has("#not-found"));

    // built from the fallback, so every outlet is new; its preload is pending
    navigate("/lazy");
    settle().await;
    navigate("/static");
    settle().await;
    load_lazy_routes().await;

    assert!(app.has("#static"));
    assert!(!app.has("#lazy"), "the abandoned route was rendered");
}

#[wasm_bindgen_test]
async fn an_abandoned_preload_does_not_replace_the_new_route_holding() {
    an_abandoned_preload_does_not_replace_the_new_route(holding).await;
}

#[wasm_bindgen_test]
async fn an_abandoned_preload_does_not_replace_the_new_route_not_holding() {
    an_abandoned_preload_does_not_replace_the_new_route(not_holding).await;
}

async fn the_initial_load_does_not_replace_a_later_fallback(
    app: fn() -> AnyView,
) {
    // the layout loads at once, its lazy child does not
    start_at("/parent/lazy");
    let app = mount(app);
    settle().await;

    navigate("/nowhere");
    settle().await;
    assert!(app.has("#not-found"));
    load_lazy_routes().await;
    assert!(
        app.has("#not-found"),
        "the initial load replaced the fallback"
    );
    assert!(!app.has("#parent") && !app.has("#lazy"));

    // and the router keeps working
    navigate("/static");
    settle().await;
    assert!(app.has("#static"));
}

#[wasm_bindgen_test]
async fn the_initial_load_does_not_replace_a_later_fallback_holding() {
    the_initial_load_does_not_replace_a_later_fallback(holding).await;
}

#[wasm_bindgen_test]
async fn the_initial_load_does_not_replace_a_later_fallback_not_holding() {
    the_initial_load_does_not_replace_a_later_fallback(not_holding).await;
}

async fn replacing_a_child_keeps_the_pending_load_of_its_layout(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    // replaces "home" with the lazy layout, whose preload is pending
    navigate("/lazy-layout/a");
    settle().await;
    // reuses the layout, replaces its child
    navigate("/lazy-layout/b");
    settle().await;
    load_lazy_routes().await;

    assert!(
        app.has("#lazy-layout"),
        "the reused layout was never rendered"
    );
    assert!(app.has("#child-b"));
    assert!(!app.has("#child-a") && !app.has("#home"));
}

#[wasm_bindgen_test]
async fn replacing_a_child_keeps_the_pending_load_of_its_layout_holding() {
    replacing_a_child_keeps_the_pending_load_of_its_layout(holding).await;
}

#[wasm_bindgen_test]
async fn replacing_a_child_keeps_the_pending_load_of_its_layout_not_holding() {
    replacing_a_child_keeps_the_pending_load_of_its_layout(not_holding).await;
}
