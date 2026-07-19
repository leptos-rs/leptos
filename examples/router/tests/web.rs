//! Regression test for `<Router set_is_routing>` with `<ProtectedRoute>`.
//!
//! When `set_is_routing` is provided, navigation must keep `is_routing` set to
//! `true` until the destination route's async resources have loaded (so the old
//! view stays visible / a progress bar can show). A plain `<Route>` does this.
//!
//! A `<ProtectedRoute>` wraps its view in a `<Transition>` + `Unsuspend`, so the
//! protected view's resources are created when the route is *built*, after the
//! router's choose-phase `AsyncTransition` has already resolved. The router
//! therefore used to clear `is_routing` immediately, exactly as if
//! `set_is_routing` were not used. It now keeps `is_routing` set until the built
//! route's suspense boundaries report (via `RouteSettleContext`) that they have
//! settled.
//!
//! `normal_route_*` is the control; `protected_route_*` guards the fix.

use futures::channel::oneshot;
use leptos::{mount::mount_to, prelude::*, task::tick};
use leptos_router::{
    components::{FlatRoutes, ProtectedRoute, Route, Router, Routes},
    hooks::use_navigate,
    NavigateOptions,
};
use leptos_router_macro::path;
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

type NavigateFn = Box<dyn Fn(&str, NavigateOptions)>;

thread_local! {
    /// Senders for the gated resources; releasing them lets the page load.
    static GATES: RefCell<Vec<oneshot::Sender<()>>> = const { RefCell::new(Vec::new()) };
    /// Receivers handed to the resource fetcher, one per built page.
    static RECEIVERS: RefCell<Vec<oneshot::Receiver<()>>> =
        const { RefCell::new(Vec::new()) };
    /// Senders for the gated auth checks; releasing them resolves `condition`.
    static AUTH_GATES: RefCell<Vec<oneshot::Sender<()>>> = const { RefCell::new(Vec::new()) };
    /// Receivers handed to the auth-check fetcher.
    static AUTH_RECEIVERS: RefCell<Vec<oneshot::Receiver<()>>> =
        const { RefCell::new(Vec::new()) };
    /// The current `<Router>`'s navigate function, grabbed from inside it.
    static NAVIGATE: RefCell<Option<NavigateFn>> = const { RefCell::new(None) };
}

/// Reset all cross-test state and the browser URL back to `/`.
fn reset() {
    GATES.with(|g| g.borrow_mut().clear());
    RECEIVERS.with(|r| r.borrow_mut().clear());
    AUTH_GATES.with(|g| g.borrow_mut().clear());
    AUTH_RECEIVERS.with(|r| r.borrow_mut().clear());
    NAVIGATE.with(|n| *n.borrow_mut() = None);
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::NULL, "", Some("/"))
        .unwrap();
}

/// Let every pending gated resource resolve.
fn release_all_gates() {
    GATES.with(|g| {
        for tx in g.borrow_mut().drain(..) {
            _ = tx.send(());
        }
    });
}

/// The number of gated page resources created so far.
fn gate_count() -> usize {
    GATES.with(|g| g.borrow().len())
}

/// Let every pending gated auth check resolve.
fn release_auth_gates() {
    AUTH_GATES.with(|g| {
        for tx in g.borrow_mut().drain(..) {
            _ = tx.send(());
        }
    });
}

fn navigate(path: &str) {
    NAVIGATE.with(|n| {
        let nav = n.borrow();
        (nav.as_ref().expect("navigate fn not set"))(
            path,
            NavigateOptions::default(),
        );
    });
}

async fn tick_n(n: usize) {
    for _ in 0..n {
        tick().await;
    }
}

fn text_of(wrapper: &web_sys::Element, selector: &str) -> Option<String> {
    wrapper
        .query_selector(selector)
        .unwrap()
        .map(|el| el.text_content().unwrap_or_default())
}

/// A page whose only async resource is gated on a `oneshot` the test controls.
/// Creating the `AsyncDerived` is what registers it with the router transition.
#[component]
fn GatedPage() -> impl IntoView {
    let (tx, rx) = oneshot::channel::<()>();
    GATES.with(|g| g.borrow_mut().push(tx));
    RECEIVERS.with(|r| r.borrow_mut().push(rx));

    let data = AsyncDerived::new(move || async move {
        let rx = RECEIVERS.with(|r| r.borrow_mut().pop());
        if let Some(rx) = rx {
            _ = rx.await;
        }
        String::from("page-data")
    });

    view! {
        <Suspense fallback=|| view! { <span id="page-fallback">"loading"</span> }>
            <span id="page">{move || Suspend::new(async move { data.await })}</span>
        </Suspense>
    }
}

/// A gated auth check: `None` until the auth gate is released, then
/// `Some(true)`. Reading it while pending suspends the boundary it is read
/// under, like a real resource-backed auth check.
fn gated_auth() -> AsyncDerived<bool> {
    let (tx, rx) = oneshot::channel::<()>();
    AUTH_GATES.with(|g| g.borrow_mut().push(tx));
    AUTH_RECEIVERS.with(|r| r.borrow_mut().push(rx));

    AsyncDerived::new(move || async move {
        let rx = AUTH_RECEIVERS.with(|r| r.borrow_mut().pop());
        if let Some(rx) = rx {
            _ = rx.await;
        }
        true
    })
}

#[component]
fn NavGrabber() -> impl IntoView {
    let nav = use_navigate();
    NAVIGATE.with(|n| *n.borrow_mut() = Some(Box::new(nav)));
}

fn router_app() -> impl IntoView {
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Router set_is_routing>
            <span id="status">
                {move || if is_routing.get() { "routing" } else { "idle" }}
            </span>
            <NavGrabber/>
            <Routes fallback=|| view! { <span>"not found"</span> }>
                <Route path=path!("") view=|| view! { <span id="home">"home"</span> }/>
                <Route path=path!("normal") view=GatedPage/>
                <ProtectedRoute
                    path=path!("protected")
                    condition=|| Some(true)
                    redirect_path=|| "/"
                    view=GatedPage
                />
            </Routes>
        </Router>
    }
}

fn flat_router_app() -> impl IntoView {
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Router set_is_routing>
            <span id="status">
                {move || if is_routing.get() { "routing" } else { "idle" }}
            </span>
            <NavGrabber/>
            <FlatRoutes fallback=|| view! { <span>"not found"</span> }>
                <Route path=path!("") view=|| view! { <span id="home">"home"</span> }/>
                <Route path=path!("normal") view=GatedPage/>
                <ProtectedRoute
                    path=path!("protected")
                    condition=|| Some(true)
                    redirect_path=|| "/"
                    view=GatedPage
                />
            </FlatRoutes>
        </Router>
    }
}

/// Like `router_app`, but the `<ProtectedRoute>` condition is itself async,
/// modeling an auth check that must load before the protected view may render.
fn async_condition_app() -> impl IntoView {
    let (is_routing, set_is_routing) = signal(false);
    let auth = gated_auth();

    view! {
        <Router set_is_routing>
            <span id="status">
                {move || if is_routing.get() { "routing" } else { "idle" }}
            </span>
            <NavGrabber/>
            <Routes fallback=|| view! { <span>"not found"</span> }>
                <Route path=path!("") view=|| view! { <span id="home">"home"</span> }/>
                <ProtectedRoute
                    path=path!("protected")
                    condition=move || auth.get()
                    redirect_path=|| "/"
                    view=GatedPage
                />
            </Routes>
        </Router>
    }
}

/// Flat-router version of [`async_condition_app`].
fn flat_async_condition_app() -> impl IntoView {
    let (is_routing, set_is_routing) = signal(false);
    let auth = gated_auth();

    view! {
        <Router set_is_routing>
            <span id="status">
                {move || if is_routing.get() { "routing" } else { "idle" }}
            </span>
            <NavGrabber/>
            <FlatRoutes fallback=|| view! { <span>"not found"</span> }>
                <Route path=path!("") view=|| view! { <span id="home">"home"</span> }/>
                <ProtectedRoute
                    path=path!("protected")
                    condition=move || auth.get()
                    redirect_path=|| "/"
                    view=GatedPage
                />
            </FlatRoutes>
        </Router>
    }
}

/// Control: a plain `<Route>` keeps `is_routing` true until its resource loads.
#[wasm_bindgen_test]
async fn normal_route_holds_is_routing_until_resources_load() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/normal");
    tick_n(20).await;

    // The gate is still closed, so the resource is pending: with `set_is_routing`
    // the router must still be in the routing state.
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a normal route should hold is_routing=true while its resource is \
         pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Same as `protected_route_*`, but through `<FlatRoutes>` rather than
/// `<Routes>`, exercising the flat-router code path.
#[wasm_bindgen_test]
async fn protected_route_in_flat_routes_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/protected");
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "ProtectedRoute in FlatRoutes should hold is_routing while its \
         resource is pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// An async `condition` must hold `is_routing` through both phases: while the
/// auth check is pending (during which the protected view — and its data
/// fetch — must not yet be created), and then while the protected view's own
/// resource loads.
#[wasm_bindgen_test]
async fn protected_route_with_async_condition_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), async_condition_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/protected");
    tick_n(20).await;

    // Phase 1: the auth check is pending. Navigation must still be in
    // progress, and the protected page must not have started fetching.
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "is_routing should be held while the async condition is pending"
    );
    assert_eq!(
        gate_count(),
        0,
        "protected data must not be fetched before the condition resolves"
    );

    release_auth_gates();
    tick_n(20).await;

    // Phase 2: the condition has resolved, so the protected view has been
    // created and its resource is now pending: still routing.
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "is_routing should still be held while the protected view's \
         resource is pending"
    );
    assert_eq!(gate_count(), 1, "the protected view should now be fetching");

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Same as `protected_route_with_async_condition_holds_is_routing`, but
/// through `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn protected_route_with_async_condition_in_flat_routes_holds_is_routing()
{
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle =
        mount_to(wrapper.clone().unchecked_into(), flat_async_condition_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/protected");
    tick_n(20).await;

    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "is_routing should be held while the async condition is pending"
    );
    assert_eq!(
        gate_count(),
        0,
        "protected data must not be fetched before the condition resolves"
    );

    release_auth_gates();
    tick_n(20).await;

    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "is_routing should still be held while the protected view's \
         resource is pending"
    );
    assert_eq!(gate_count(), 1, "the protected view should now be fetching");

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Bug: a `<ProtectedRoute>` does *not* hold `is_routing` for its resources.
#[wasm_bindgen_test]
async fn protected_route_holds_is_routing_until_resources_load() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/protected");
    tick_n(20).await;

    // Same scenario as the control, but for a ProtectedRoute. Today this fails:
    // is_routing has already flipped back to "idle" while the resource is still
    // pending, i.e. navigation behaves as if set_is_routing were not used.
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "ProtectedRoute should hold is_routing=true while its resource is \
         pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}
