#![cfg(target_arch = "wasm32")]

//! With `<Router set_is_routing>`, a navigation keeps the previous page on
//! screen, and `is_routing` set, until the new page is ready; the tests run
//! every scenario through both `<Routes>` and `<FlatRoutes>`.

mod common;

use common::*;
use leptos::prelude::*;
use leptos_router::{
    components::{
        FlatRoutes, Outlet, ProtectedParentRoute, ProtectedRoute, Route,
        Router, Routes,
    },
    path,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static CONDITION: RefCell<Option<WriteSignal<Option<bool>>>> =
        const { RefCell::new(None) };
    static DISPOSED_PAGES: Cell<usize> = const { Cell::new(0) };
}

/// Sets the condition of the `/signal-protected` route.
fn set_condition(value: Option<bool>) {
    CONDITION.with(|condition| {
        condition
            .borrow()
            .expect("the app has no /signal-protected route")
            .set(value)
    });
}

/// How many `<Page/>`s have been disposed of.
fn disposed_pages() -> usize {
    DISPOSED_PAGES.with(Cell::get)
}

/// A page whose resource, created in its body, waits for the "page" gate.
#[component]
fn Page() -> impl IntoView {
    on_cleanup(|| DISPOSED_PAGES.with(|count| count.set(count.get() + 1)));
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

/// A resource the app creates, which waits for the "late" gate.
#[derive(Clone, Copy)]
struct Late(AsyncDerived<&'static str>);

#[derive(Clone, Copy)]
struct PageContext(&'static str);

/// A page that provides a context to its children, and reads `Late` while it
/// renders, outside any `<Suspense>` of its own.
#[component]
fn ContextPage() -> impl IntoView {
    provide_context(PageContext("page context"));
    let Late(late) = expect_context();
    view! {
        <ContextChild/>
        <p id="late">{move || late.get()}</p>
    }
}

#[component]
fn ContextChild() -> impl IntoView {
    view! {
        <p id="context">
            {move || use_context::<PageContext>().map(|c| c.0).unwrap_or("missing")}
        </p>
    }
}

/// A layout that provides a context to its child routes, and whose resource,
/// created in its body, waits for the "parent" gate.
#[component]
fn ParentPage() -> impl IntoView {
    provide_context(PageContext("parent context"));
    let data = AsyncDerived::new(|| {
        let loaded = gate("parent");
        async move {
            loaded.await;
            "parent"
        }
    });
    view! {
        <Suspense>
            <p id="parent">{move || Suspend::new(async move { data.await })}</p>
        </Suspense>
        <Outlet/>
    }
}

#[component]
fn ChildPage() -> impl IntoView {
    view! {
        <p id="child">
            {move || use_context::<PageContext>().map(|c| c.0).unwrap_or("missing")}
        </p>
    }
}

/// An access check that waits for the gate named `name`, then says `allowed`.
fn access_check(name: &'static str, allowed: bool) -> AsyncDerived<bool> {
    AsyncDerived::new(move || {
        let checked = gate(name);
        async move {
            checked.await;
            allowed
        }
    })
}

fn protected_fallback() -> impl IntoView {
    view! { <p id="protected-fallback">"checking"</p> }
}

/// The same app, with `set_is_routing` (`holding`) or without it, rendering
/// its routes through `<Routes>` or `<FlatRoutes>` (`$routes`); `$extra` are
/// routes only `<Routes>` supports.
macro_rules! app {
    ($routes:ident, $($holding:ident)?, { $($extra:tt)* }) => {{
        let auth = access_check("auth", true);
        let deny = access_check("deny", false);
        let (condition, set_condition) = signal(None::<bool>);
        CONDITION.with(|c| *c.borrow_mut() = Some(set_condition));
        provide_context(Late(AsyncDerived::new(|| {
            let loaded = gate("late");
            async move {
                loaded.await;
                "late"
            }
        })));
        // built inside <Router>, whose context its components need
        let routes = move || view! {
            <CaptureNavigate/>
            <a id="to-page" href="/page">"page"</a>
            <a id="to-protected" href="/protected">"protected"</a>
            <$routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                <Route path=path!("page") view=Page/>
                <Route path=path!("login") view=|| view! { <p id="login">"login"</p> }/>
                <ProtectedRoute
                    path=path!("protected")
                    condition=|| Some(true)
                    redirect_path=|| "/login"
                    view=Page
                />
                <ProtectedRoute
                    path=path!("auth-protected")
                    condition=move || auth.get()
                    redirect_path=|| "/login"
                    view=Page
                />
                <ProtectedRoute
                    path=path!("denied")
                    condition=move || deny.get()
                    redirect_path=|| "/login"
                    view=Page
                />
                <ProtectedRoute
                    path=path!("signal-protected")
                    condition=move || condition.get()
                    redirect_path=|| "/login"
                    view=Page
                    fallback=protected_fallback
                />
                <ProtectedRoute
                    path=path!("contexts")
                    condition=|| Some(true)
                    redirect_path=|| "/login"
                    view=ContextPage
                    fallback=protected_fallback
                />
                $($extra)*
            </$routes>
        };
        app!(@router routes, $($holding)?)
    }};
    (@router $routes:ident, holding) => {
        view! { <Router set_is_routing=routing_setter()>{$routes()}</Router> }
    };
    (@router $routes:ident,) => {
        view! { <Router>{$routes()}</Router> }
    };
}

fn nested() -> impl IntoView {
    app!(Routes, holding, {
        <ProtectedParentRoute
            path=path!("parent")
            condition=|| Some(true)
            redirect_path=|| "/login"
            view=ParentPage
        >
            <Route path=path!("child") view=ChildPage/>
        </ProtectedParentRoute>
    })
}

fn flat() -> impl IntoView {
    app!(FlatRoutes, holding, {})
}

fn nested_not_holding() -> impl IntoView {
    app!(Routes, , {})
}

fn flat_not_holding() -> impl IntoView {
    app!(FlatRoutes, , {})
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

async fn a_protected_route_holds_the_previous_page_until_its_content_has_loaded(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/protected");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());
    assert_eq!(pending("page"), 1, "the content has been created");

    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!app.has("#home"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_protected_route_holds_the_previous_page_until_its_content_has_loaded()
 {
    a_protected_route_holds_the_previous_page_until_its_content_has_loaded(
        || nested().into_any(),
    )
    .await;
}

#[wasm_bindgen_test]
async fn flat_protected_route_holds_the_previous_page_until_its_content_has_loaded()
 {
    a_protected_route_holds_the_previous_page_until_its_content_has_loaded(
        || flat().into_any(),
    )
    .await;
}

async fn a_protected_route_waits_for_its_condition(app: fn() -> AnyView) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/auth-protected");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());
    assert_eq!(pending("page"), 0, "no content before access is granted");

    release("auth");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());
    assert_eq!(pending("page"), 1, "the content has been created");

    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_protected_route_waits_for_its_condition() {
    a_protected_route_waits_for_its_condition(|| nested().into_any()).await;
}

#[wasm_bindgen_test]
async fn flat_protected_route_waits_for_its_condition() {
    a_protected_route_waits_for_its_condition(|| flat().into_any()).await;
}

async fn a_denied_protected_route_redirects_without_creating_its_content(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/denied");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());

    release("deny");
    sleep(50).await;
    assert!(app.has("#login"));
    assert_eq!(pathname(), "/login");
    assert_eq!(pending("page"), 0, "no content without access");
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_denied_protected_route_redirects_without_creating_its_content()
{
    a_denied_protected_route_redirects_without_creating_its_content(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_denied_protected_route_redirects_without_creating_its_content() {
    a_denied_protected_route_redirects_without_creating_its_content(|| {
        flat().into_any()
    })
    .await;
}

/// A condition that reads no resource cannot be waited for: the route shows
/// its fallback, as without `set_is_routing`, and does not hold anything.
async fn a_protected_route_with_an_unknown_condition_is_not_held(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/signal-protected");
    settle().await;
    assert!(app.has("#protected-fallback"));
    assert!(!is_routing());

    set_condition(Some(true));
    settle().await;
    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
}

#[wasm_bindgen_test]
async fn nested_protected_route_with_an_unknown_condition_is_not_held() {
    a_protected_route_with_an_unknown_condition_is_not_held(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_protected_route_with_an_unknown_condition_is_not_held() {
    a_protected_route_with_an_unknown_condition_is_not_held(|| {
        flat().into_any()
    })
    .await;
}

async fn protected_content_is_disposed_of_when_access_is_revoked(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;
    set_condition(Some(true));

    navigate("/signal-protected");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));

    let disposed = disposed_pages();
    set_condition(Some(false));
    sleep(50).await;
    assert!(app.has("#login"));
    assert_eq!(
        disposed_pages(),
        disposed + 1,
        "the content was not disposed"
    );
}

#[wasm_bindgen_test]
async fn nested_protected_content_is_disposed_of_when_access_is_revoked() {
    protected_content_is_disposed_of_when_access_is_revoked(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_protected_content_is_disposed_of_when_access_is_revoked() {
    protected_content_is_disposed_of_when_access_is_revoked(|| {
        flat().into_any()
    })
    .await;
}

/// Protected content still provides contexts to its children, and still
/// suspends the protected route's own `<Transition>` while it reads a
/// resource that is loading.
async fn protected_content_keeps_its_contexts_and_transition(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/contexts");
    settle().await;
    assert!(app.has("#protected-fallback"));
    assert!(!app.has("#home"));

    release("late");
    settle().await;
    assert_eq!(app.text("#late").as_deref(), Some("late"));
    assert_eq!(app.text("#context").as_deref(), Some("page context"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_protected_content_keeps_its_contexts_and_transition() {
    protected_content_keeps_its_contexts_and_transition(|| nested().into_any())
        .await;
}

#[wasm_bindgen_test]
async fn flat_protected_content_keeps_its_contexts_and_transition() {
    protected_content_keeps_its_contexts_and_transition(|| flat().into_any())
        .await;
}

#[wasm_bindgen_test]
async fn a_protected_parent_route_holds_the_previous_page_until_its_content_has_loaded()
 {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/parent/child");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());

    release("parent");
    settle().await;
    assert_eq!(app.text("#parent").as_deref(), Some("parent"));
    assert_eq!(app.text("#child").as_deref(), Some("parent context"));
    assert!(!is_routing());
}

async fn without_set_is_routing_a_protected_route_is_shown_at_once(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/protected");
    settle().await;
    assert!(!app.has("#home"));
    assert!(app.has("#page-fallback"));
}

#[wasm_bindgen_test]
async fn nested_protected_route_without_set_is_routing_is_shown_at_once() {
    without_set_is_routing_a_protected_route_is_shown_at_once(|| {
        nested_not_holding().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_protected_route_without_set_is_routing_is_shown_at_once() {
    without_set_is_routing_a_protected_route_is_shown_at_once(|| {
        flat_not_holding().into_any()
    })
    .await;
}
