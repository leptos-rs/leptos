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
    components::{
        FlatRoutes, Outlet, ParentRoute, ProtectedRoute, Route, Router, Routes,
    },
    hooks::{use_navigate, use_params_map},
    Lazy, LazyRoute, NavigateOptions,
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
    /// Setter that mounts a second `<GatedPage>` inside `<LatePage>`.
    static SHOW_LATE: RefCell<Option<WriteSignal<bool>>> = const { RefCell::new(None) };
    /// Senders for `LazyPage`'s preload and view; a navigation to it stays in
    /// flight until they are released (see [`release_lazy_gates`]).
    static LAZY_GATES: RefCell<Vec<oneshot::Sender<()>>> = const { RefCell::new(Vec::new()) };
}

/// Reset all cross-test state and the browser URL back to `/`.
fn reset() {
    GATES.with(|g| g.borrow_mut().clear());
    RECEIVERS.with(|r| r.borrow_mut().clear());
    AUTH_GATES.with(|g| g.borrow_mut().clear());
    AUTH_RECEIVERS.with(|r| r.borrow_mut().clear());
    NAVIGATE.with(|n| *n.borrow_mut() = None);
    SHOW_LATE.with(|s| *s.borrow_mut() = None);
    LAZY_GATES.with(|g| g.borrow_mut().clear());
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

/// Release only the oldest pending gated resource.
fn release_first_gate() {
    GATES.with(|g| {
        let mut g = g.borrow_mut();
        if !g.is_empty() {
            _ = g.remove(0).send(());
        }
    });
}

/// Let every pending gated auth check resolve.
fn release_auth_gates() {
    AUTH_GATES.with(|g| {
        for tx in g.borrow_mut().drain(..) {
            _ = tx.send(());
        }
    });
}

/// Mount a second gated page inside the current `<LatePage>`.
fn show_late() {
    SHOW_LATE.with(|s| {
        s.borrow()
            .as_ref()
            .expect("show_late setter not set")
            .set(true);
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

/// A page that reads a gated resource *outside* any `<Suspense>`, so only
/// the choose-phase `AsyncTransition` can hold `is_routing` for it.
#[component]
fn UnboundedPage() -> impl IntoView {
    let (tx, rx) = oneshot::channel::<()>();
    GATES.with(|g| g.borrow_mut().push(tx));
    RECEIVERS.with(|r| r.borrow_mut().push(rx));

    let data = AsyncDerived::new(move || async move {
        let rx = RECEIVERS.with(|r| r.borrow_mut().pop());
        if let Some(rx) = rx {
            _ = rx.await;
        }
        String::from("unbounded-data")
    });

    view! { <span id="unbounded">{move || data.get().unwrap_or_default()}</span> }
}

/// A page whose resource depends on the `:id` route param and is gated on a
/// `oneshot` the test controls; every param change creates a new gate. It
/// also renders a `<GatedPage/>`, whose resource does not depend on the
/// params.
#[component]
fn ParamPage() -> impl IntoView {
    let params = use_params_map();
    let data = AsyncDerived::new(move || {
        let id = params.with(|p| p.get("id").unwrap_or_default());
        let (tx, rx) = oneshot::channel::<()>();
        GATES.with(|g| g.borrow_mut().push(tx));
        async move {
            _ = rx.await;
            format!("item-{id}")
        }
    });

    view! {
        <Transition fallback=|| view! { <span id="param-fallback">"loading"</span> }>
            <span id="param">{move || Suspend::new(async move { data.await })}</span>
        </Transition>
        // a resource that does not depend on the params
        <GatedPage/>
    }
}

/// Mounts a gated boundary on demand (see [`show_late`]), to model a boundary
/// created by user interaction rather than by a navigation.
#[component]
fn LateGate() -> impl IntoView {
    let (show, set_show) = signal(false);
    SHOW_LATE.with(|s| *s.borrow_mut() = Some(set_show));

    move || {
        show.get()
            .then(|| view! { <span id="late"><GatedPage/></span> })
    }
}

/// Like [`ParamPage`], but the unrelated gated resource is only created
/// while the page renders, after its param resource has loaded: it does not
/// exist yet when the route's view has been chosen.
#[component]
fn DeferredPage() -> impl IntoView {
    let params = use_params_map();
    let data = AsyncDerived::new(move || {
        let id = params.with(|p| p.get("id").unwrap_or_default());
        let (tx, rx) = oneshot::channel::<()>();
        GATES.with(|g| g.borrow_mut().push(tx));
        async move {
            _ = rx.await;
            format!("item-{id}")
        }
    });

    view! {
        <Transition fallback=|| view! { <span id="param-fallback">"loading"</span> }>
            {move || Suspend::new(async move {
                let value = data.await;
                view! { <span id="param">{value}</span><GatedPage/> }
            })}
        </Transition>
    }
}

/// A page that can mount a second gated boundary after it has loaded.
#[component]
fn LatePage() -> impl IntoView {
    view! {
        <GatedPage/>
        <LateGate/>
    }
}

/// A lazy route whose preload (awaited by the nested router) and view
/// (awaited by the flat router) both wait on a gate the test controls.
struct LazyPage;

fn lazy_gate() -> impl std::future::Future<Output = ()> {
    let (tx, rx) = oneshot::channel::<()>();
    LAZY_GATES.with(|g| g.borrow_mut().push(tx));
    async move {
        _ = rx.await;
    }
}

fn lazy_page() -> Lazy<LazyPage> {
    Lazy::new()
}

impl LazyRoute for LazyPage {
    fn data() -> Self {
        LazyPage
    }

    async fn view(_this: Self) -> AnyView {
        lazy_gate().await;
        view! { <span id="lazy">"lazy"</span> }.into_any()
    }

    async fn preload() {
        lazy_gate().await;
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
                <Route path=path!("late") view=LatePage/>
                <Route path=path!("lazy") view=lazy_page()/>
                <Route path=path!("lazy/:id") view=lazy_page()/>
                <Route path=path!("items/:id") view=ParamPage/>
                <Route path=path!("deferred/:id") view=DeferredPage/>
                <Route path=path!("unbounded") view=UnboundedPage/>
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
                <Route path=path!("late") view=LatePage/>
                <Route path=path!("lazy") view=lazy_page()/>
                <Route path=path!("lazy/:id") view=lazy_page()/>
                <Route path=path!("items/:id") view=ParamPage/>
                <Route path=path!("deferred/:id") view=DeferredPage/>
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

/// Nested layouts, to exercise navigations that build outlets at a depth that
/// did not exist before the navigation.
fn nested_router_app() -> impl IntoView {
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Router set_is_routing>
            <span id="status">
                {move || if is_routing.get() { "routing" } else { "idle" }}
            </span>
            <NavGrabber/>
            <Routes fallback=|| view! { <span>"not found"</span> }>
                <Route path=path!("") view=|| view! { <span id="home">"home"</span> }/>
                <ParentRoute
                    path=path!("parent")
                    view=|| view! { <span id="parent">"parent"</span><LateGate/><Outlet/> }
                >
                    <Route path=path!("") view=|| view! { <span id="parent-index">"index"</span> }/>
                    <ParentRoute
                        path=path!("child")
                        view=|| view! { <span id="child">"child"</span><Outlet/> }
                    >
                        <Route path=path!("") view=GatedPage/>
                    </ParentRoute>
                </ParentRoute>
            </Routes>
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
    let _handle =
        mount_to(wrapper.clone().unchecked_into(), async_condition_app);

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
        "is_routing should still be held while the protected view's resource \
         is pending"
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
        "is_routing should still be held while the protected view's resource \
         is pending"
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

/// A navigation that is superseded by a newer one must not clear `is_routing`
/// while the newer navigation is still loading: the newest navigation is the
/// one responsible for clearing it.
#[wasm_bindgen_test]
async fn superseded_navigation_does_not_clear_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    // First navigation: the protected page is built and waiting on its gate.
    navigate("/protected");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    assert_eq!(gate_count(), 1);

    // Second navigation supersedes the first while it is still pending.
    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 2);

    // Releasing only the first page's gate settles the superseded navigation;
    // it must not clear is_routing while the second is still loading.
    release_first_gate();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation must not clear is_routing while the newest \
         navigation is still loading"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Same as `superseded_navigation_does_not_clear_is_routing`, but through
/// `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn superseded_navigation_in_flat_routes_does_not_clear_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/protected");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    assert_eq!(gate_count(), 1);

    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 2);

    release_first_gate();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation must not clear is_routing while the newest \
         navigation is still loading"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Navigating to an unmatched URL (the fallback) while another navigation is
/// still loading must clear `is_routing`: the fallback renders immediately,
/// and the superseded navigation no longer clears the flag itself.
#[wasm_bindgen_test]
async fn navigation_to_fallback_clears_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/normal");
    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));

    navigate("/does-not-exist");
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("idle"),
        "navigating to the fallback must clear is_routing even if a \
         superseded navigation is still pending"
    );

    // releasing the abandoned page's gate must not bring routing back
    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
}

/// Same as `navigation_to_fallback_clears_is_routing`, but through
/// `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn navigation_to_fallback_in_flat_routes_clears_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/normal");
    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));

    navigate("/does-not-exist");
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("idle"),
        "navigating to the fallback must clear is_routing even if a \
         superseded navigation is still pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
}

/// A boundary created after the navigation has settled (e.g. by user
/// interaction inside the route) must not register with the navigation's
/// `RouteSettleContext`, which it can still find via `use_context`.
#[wasm_bindgen_test]
async fn boundary_created_after_settle_does_not_register() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    navigate("/late");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    show_late();
    tick_n(20).await;
    // the late boundary is mounted and pending on its gate
    assert_eq!(
        text_of(&wrapper, "#late #page-fallback").as_deref(),
        Some("loading")
    );
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("idle"),
        "a boundary created after the navigation settled must not hold \
         is_routing"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(
        text_of(&wrapper, "#late #page").as_deref(),
        Some("page-data")
    );
}

/// Same as `boundary_created_after_settle_does_not_register`, but through
/// `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn boundary_created_after_settle_in_flat_routes_does_not_register() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    navigate("/late");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    show_late();
    tick_n(20).await;
    // the late boundary is mounted and pending on its gate
    assert_eq!(
        text_of(&wrapper, "#late #page-fallback").as_deref(),
        Some("loading")
    );
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("idle"),
        "a boundary created after the navigation settled must not hold \
         is_routing"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(
        text_of(&wrapper, "#late #page").as_deref(),
        Some("page-data")
    );
}

/// Navigating from the fallback to a route builds the route's outlets from
/// scratch. `is_routing` must still be held until the route has loaded.
#[wasm_bindgen_test]
async fn navigation_from_fallback_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    navigate("/does-not-exist");
    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 1);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "navigating from the fallback must hold is_routing while the route's \
         resource is pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Same as `navigation_from_fallback_holds_is_routing`, but through
/// `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn navigation_from_fallback_in_flat_routes_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    navigate("/does-not-exist");
    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 1);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "navigating from the fallback must hold is_routing while the route's \
         resource is pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Navigating deeper into a nested layout builds outlets below the level that
/// changed from scratch. `is_routing` must be held until the deepest route has
/// loaded, not only until the swapped level has been chosen.
#[wasm_bindgen_test]
async fn navigation_to_deeper_outlets_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), nested_router_app);

    tick_n(10).await;
    navigate("/parent");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#parent-index").as_deref(), Some("index"));

    navigate("/parent/child");
    tick_n(20).await;
    assert_eq!(gate_count(), 1);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "navigating to a deeper outlet must hold is_routing while its \
         resource is pending"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Navigating A -> B -> A while the first navigation to A is still in flight:
/// the first navigation must not clear `is_routing` for the second one when
/// it completes, even though both target the same route.
#[wasm_bindgen_test]
async fn returning_to_a_superseded_route_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    navigate("/normal");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    assert_eq!(gate_count(), 1);

    // Supersede the first navigation with one that never finishes loading,
    // then navigate back to the first route while the first navigation is
    // still in flight.
    navigate("/lazy");
    tick_n(10).await;
    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 2);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation to the same route must not clear is_routing \
         while the newest navigation is still loading"
    );

    // Releasing only the first navigation's gate settles it; it must still
    // not clear is_routing for the newest navigation.
    release_first_gate();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation to the same route must not clear is_routing \
         while the newest navigation is still loading"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// Same as `returning_to_a_superseded_route_holds_is_routing`, but through
/// `<FlatRoutes>`.
#[wasm_bindgen_test]
async fn returning_to_a_superseded_route_in_flat_routes_holds_is_routing() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    navigate("/normal");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    assert_eq!(gate_count(), 1);

    // Supersede the first navigation with one that never finishes loading,
    // then navigate back to the first route while the first navigation is
    // still in flight.
    navigate("/lazy");
    tick_n(10).await;
    navigate("/normal");
    tick_n(20).await;
    assert_eq!(gate_count(), 2);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation to the same route must not clear is_routing \
         while the newest navigation is still loading"
    );

    // Releasing only the first navigation's gate settles it; it must still
    // not clear is_routing for the newest navigation.
    release_first_gate();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a superseded navigation to the same route must not clear is_routing \
         while the newest navigation is still loading"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
}

/// A superseded navigation must not dispose the view that is still on
/// screen: its reactivity has to survive until the newest navigation renders.
#[wasm_bindgen_test]
async fn superseded_navigation_keeps_displayed_view_reactive() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), flat_router_app);

    tick_n(10).await;
    navigate("/late");
    tick_n(20).await;
    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));

    // a navigation held in flight by its gate, superseded by one that never
    // finishes, then released: it completes without ever being displayed
    navigate("/normal");
    tick_n(10).await;
    assert_eq!(gate_count(), 1);
    navigate("/lazy");
    tick_n(10).await;
    release_first_gate();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));

    show_late();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#late #page-fallback").as_deref(),
        Some("loading"),
        "the displayed view must still react after a superseded navigation \
         completed"
    );
}

/// A boundary created by user interaction inside a *retained* parent route
/// while a child navigation is loading is not part of that navigation and
/// must not hold `is_routing`.
#[wasm_bindgen_test]
async fn boundary_created_in_retained_parent_does_not_register() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), nested_router_app);

    tick_n(10).await;
    navigate("/parent");
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    // the child navigation is pending on its gate...
    navigate("/parent/child");
    tick_n(20).await;
    assert_eq!(gate_count(), 1);
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("routing"));

    // ...when the user opens a gated boundary in the retained parent
    show_late();
    tick_n(20).await;
    assert_eq!(gate_count(), 2);
    assert_eq!(
        text_of(&wrapper, "#late #page-fallback").as_deref(),
        Some("loading")
    );

    // loading the child alone must complete the navigation
    release_first_gate();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#page").as_deref(), Some("page-data"));
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("idle"),
        "a boundary created in a retained parent must not hold is_routing"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(
        text_of(&wrapper, "#late #page").as_deref(),
        Some("page-data")
    );
}

/// An outlet built from scratch (here: from the fallback) must hold
/// `is_routing` for resources created while its view is chosen, exactly like
/// a swapped outlet does, even when no boundary reads them.
#[wasm_bindgen_test]
async fn fresh_outlet_holds_is_routing_for_unbounded_resources() {
    reset();
    let document = document();
    let wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&wrapper).unwrap();
    let _handle = mount_to(wrapper.clone().unchecked_into(), router_app);

    tick_n(10).await;
    navigate("/does-not-exist");
    tick_n(10).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));

    navigate("/unbounded");
    tick_n(20).await;
    assert_eq!(gate_count(), 1);
    assert_eq!(
        text_of(&wrapper, "#status").as_deref(),
        Some("routing"),
        "a freshly built outlet must hold is_routing for resources created \
         while its view is chosen"
    );

    release_all_gates();
    tick_n(20).await;
    assert_eq!(text_of(&wrapper, "#status").as_deref(), Some("idle"));
    assert_eq!(
        text_of(&wrapper, "#unbounded").as_deref(),
        Some("unbounded-data")
    );
}
