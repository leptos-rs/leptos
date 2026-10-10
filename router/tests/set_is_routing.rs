#![cfg(target_arch = "wasm32")]

//! With `<Router set_is_routing>`, a navigation keeps the previous page on
//! screen, and `is_routing` set, until the new page is ready; the tests run
//! every scenario through both `<Routes>` and `<FlatRoutes>`.

mod common;

use any_spawner::Executor;
use common::*;
use leptos::prelude::*;
use leptos_router::{
    Lazy, LazyRoute,
    components::{
        FlatRoutes, Outlet, ParentRoute, ProtectedParentRoute, ProtectedRoute,
        Route, Router, Routes,
    },
    hooks::use_params_map,
    path,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static CONDITION: RefCell<Option<WriteSignal<Option<bool>>>> =
        const { RefCell::new(None) };
    static SWITCH: RefCell<Option<WriteSignal<bool>>> =
        const { RefCell::new(None) };
    static DISPOSED_PAGES: Cell<usize> = const { Cell::new(0) };
    static PAGE_SHOWN_WHEN_ROUTED: RefCell<Vec<bool>> =
        const { RefCell::new(Vec::new()) };
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

/// Sets the switch that the view of `/switch-layout/switch` reads.
fn set_switch(value: bool) {
    SWITCH.with(|switch| {
        switch
            .borrow()
            .expect("the app has no /switch-layout/switch route")
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

/// Like [`Page`], with the "other" gate.
#[component]
fn OtherPage() -> impl IntoView {
    let data = AsyncDerived::new(|| {
        let loaded = gate("other");
        async move {
            loaded.await;
            "loaded"
        }
    });
    view! {
        <Suspense>
            <p id="other">{move || Suspend::new(async move { data.await })}</p>
        </Suspense>
    }
}

/// A page whose resource depends on the `:id` param, and waits for the "item"
/// gate every time it loads.
#[component]
fn ItemPage() -> impl IntoView {
    let params = use_params_map();
    let item = AsyncDerived::new(move || {
        let id = params.with(|params| params.get("id").unwrap_or_default());
        let loaded = gate("item");
        async move {
            loaded.await;
            format!("item {id}")
        }
    });
    view! {
        <Transition>
            <p id="item">{move || Suspend::new(async move { item.await })}</p>
        </Transition>
    }
}

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

fn lazy_page() -> Lazy<LazyPage> {
    Lazy::new()
}

/// Lets a pending lazy route load: its preload (with `<Routes>`), then its
/// view.
async fn load_lazy_page() {
    release("lazy");
    settle().await;
    release("lazy");
    settle().await;
}

/// A layout that does not render its child routes, as a layout may do on
/// small screens, or before the user has opened a panel.
#[component]
fn HiddenOutletLayout() -> impl IntoView {
    view! {
        <p id="hidden-outlet">"layout"</p>
        <Show when=|| false>
            <Outlet/>
        </Show>
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

/// A layout that provides a context to its child routes only after it has
/// created its view.
#[component]
fn LateContextLayout() -> impl IntoView {
    let view = view! { <p id="late-context">"layout"</p><Outlet/> };
    provide_context(PageContext("late context"));
    view
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
            <a id="to-other" href="/other">"other"</a>
            <a id="to-item-2" href="/items/2">"item 2"</a>
            <a id="to-lazy-1" href="/lazy/1">"lazy 1"</a>
            <a id="to-lazy-2" href="/lazy/2">"lazy 2"</a>
            <$routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                <Route path=path!("page") view=Page/>
                <Route path=path!("other") view=OtherPage/>
                <Route path=path!("items/:id") view=ItemPage/>
                <Route path=path!("lazy/:id") view=lazy_page()/>
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
    let (switch, set_switch) = signal(false);
    SWITCH.with(|s| *s.borrow_mut() = Some(set_switch));
    app!(Routes, holding, {
        <ParentRoute path=path!("app") view=|| view! { <p id="app">"app"</p><Outlet/> }>
            <ProtectedRoute
                path=path!("protected")
                condition=|| Some(true)
                redirect_path=|| "/login"
                view=Page
            />
        </ParentRoute>
        <ParentRoute path=path!("late-context") view=LateContextLayout>
            <Route path=path!("child") view=ChildPage/>
        </ParentRoute>
        <ParentRoute
            path=path!("switch-layout")
            view=|| view! { <p id="switch-layout">"layout"</p><Outlet/> }
        >
            <Route
                path=path!("switch")
                view=move || if switch.get() {
                    view! { <p id="switch">"on"</p> }
                } else {
                    view! { <p id="switch">"off"</p> }
                }
            />
        </ParentRoute>
        <ProtectedParentRoute
            path=path!("parent")
            condition=|| Some(true)
            redirect_path=|| "/login"
            view=ParentPage
        >
            <Route path=path!("child") view=ChildPage/>
        </ProtectedParentRoute>
        <ParentRoute
            path=path!("no-outlet")
            view=|| view! { <p id="no-outlet">"layout"</p> }
        >
            <Route path=path!("x") view=|| view! { <p id="x">"x"</p> }/>
            <Route path=path!("y") view=|| view! { <p id="y">"y"</p> }/>
        </ParentRoute>
        <ParentRoute
            path=path!("layout")
            view=|| view! { <p id="layout">"layout"</p><Outlet/> }
        >
            <Route path=path!("page") view=Page/>
            <Route path=path!("other") view=OtherPage/>
            <Route path=path!("lazy") view=lazy_page()/>
        </ParentRoute>
        <ParentRoute
            path=path!("twice")
            view=|| view! {
                <div id="first"><Outlet/></div>
                <div id="second"><Outlet/></div>
            }
        >
            <Route path=path!("page") view=Page/>
            <Route path=path!("other") view=OtherPage/>
        </ParentRoute>
        <ParentRoute path=path!("hidden-outlet") view=HiddenOutletLayout>
            <Route path=path!("x") view=|| view! { <p id="x">"x"</p> }/>
            <Route path=path!("y") view=|| view! { <p id="y">"y"</p> }/>
        </ParentRoute>
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

async fn an_earlier_navigation_does_not_complete_a_later_one(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    app.click("#to-page");
    settle().await;
    app.click("#to-other");
    settle().await;
    release("page");
    settle().await;
    assert!(is_routing(), "the later navigation is still loading");
    assert!(app.has("#home"), "the previous page is held");
    assert_eq!(pathname(), "/", "the URL changes with the page");

    release("other");
    settle().await;
    assert_eq!(app.text("#other").as_deref(), Some("loaded"));
    assert_eq!(pathname(), "/other");
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_earlier_navigation_does_not_complete_a_later_one() {
    an_earlier_navigation_does_not_complete_a_later_one(|| nested().into_any())
        .await;
}

#[wasm_bindgen_test]
async fn flat_earlier_navigation_does_not_complete_a_later_one() {
    an_earlier_navigation_does_not_complete_a_later_one(|| flat().into_any())
        .await;
}

async fn returning_to_a_route_does_not_show_its_abandoned_load(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/page");
    settle().await;
    navigate("/other");
    settle().await;
    navigate("/page");
    settle().await;
    // the first navigation to /page finishes loading
    release_first("page");
    settle().await;
    assert!(!app.has("#page"), "the abandoned load was shown");
    assert!(is_routing());

    release("page");
    release("other");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_returning_to_a_route_does_not_show_its_abandoned_load() {
    returning_to_a_route_does_not_show_its_abandoned_load(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_returning_to_a_route_does_not_show_its_abandoned_load() {
    returning_to_a_route_does_not_show_its_abandoned_load(|| flat().into_any())
        .await;
}

async fn navigating_to_the_fallback_clears_is_routing(app: fn() -> AnyView) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/page");
    settle().await;
    assert!(is_routing());
    navigate("/nowhere");
    settle().await;
    assert!(app.has("#not-found"));
    assert!(!is_routing());

    // the abandoned navigation finishes loading
    release("page");
    settle().await;
    assert!(app.has("#not-found"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_navigating_to_the_fallback_clears_is_routing() {
    navigating_to_the_fallback_clears_is_routing(|| nested().into_any()).await;
}

#[wasm_bindgen_test]
async fn flat_navigating_to_the_fallback_clears_is_routing() {
    navigating_to_the_fallback_clears_is_routing(|| flat().into_any()).await;
}

/// A navigation that only replaces the child of a layout that does not render
/// its child routes has nothing to show, and must not wait for it.
#[wasm_bindgen_test]
async fn a_child_route_nothing_renders_does_not_hold_the_navigation() {
    for layout in ["no-outlet", "hidden-outlet"] {
        start_at("/");
        let app = mount(|| nested().into_any());
        settle().await;

        navigate(&format!("/{layout}/x"));
        settle().await;
        assert!(app.has(&format!("#{layout}")));
        assert!(!is_routing());

        navigate(&format!("/{layout}/y"));
        settle().await;
        assert!(!is_routing(), "{layout}: the navigation never completed");
    }
}

/// A navigation that only replaces the child of a layout still waits for the
/// new child's view, which the layout renders.
#[wasm_bindgen_test]
async fn replacing_the_child_of_a_layout_holds_the_previous_child() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;
    navigate("/layout/page");
    settle().await;
    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!is_routing());

    navigate("/layout/other");
    settle().await;
    assert!(
        app.has("#layout") && app.has("#page"),
        "the previous child is held"
    );
    assert!(is_routing());

    release("other");
    settle().await;
    assert_eq!(app.text("#other").as_deref(), Some("loaded"));
    assert!(!app.has("#page"));
    assert!(!is_routing());
}

async fn holds_the_previous_page_until_the_outlets_it_adds_have_loaded(
    path: &str,
    layout: &str,
) {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate(path);
    settle().await;
    assert!(app.has("#home"), "{path}: the previous page is held");
    assert!(!app.has(layout));
    assert!(is_routing());

    release("page");
    settle().await;
    assert!(app.has(layout));
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!app.has("#home"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn a_navigation_holds_the_previous_page_until_the_outlets_it_adds_have_loaded()
 {
    holds_the_previous_page_until_the_outlets_it_adds_have_loaded(
        "/layout/page",
        "#layout",
    )
    .await;
}

#[wasm_bindgen_test]
async fn a_protected_route_under_a_new_layout_holds_the_previous_page() {
    holds_the_previous_page_until_the_outlets_it_adds_have_loaded(
        "/app/protected",
        "#app",
    )
    .await;
}

#[wasm_bindgen_test]
async fn a_navigation_from_the_fallback_holds_the_fallback() {
    start_at("/nowhere");
    let app = mount(|| nested().into_any());
    settle().await;
    assert!(app.has("#not-found"));

    navigate("/layout/page");
    settle().await;
    assert!(app.has("#not-found"), "the fallback is held");
    assert!(is_routing());

    release("page");
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert!(!app.has("#not-found"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn a_layout_provides_context_to_the_outlets_a_navigation_adds() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/late-context/child");
    settle().await;
    assert_eq!(app.text("#child").as_deref(), Some("late context"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn a_route_view_added_by_a_navigation_stays_reactive() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/switch-layout/switch");
    settle().await;
    assert_eq!(app.text("#switch").as_deref(), Some("off"));
    set_switch(true);
    settle().await;
    assert_eq!(app.text("#switch").as_deref(), Some("on"));
}

#[wasm_bindgen_test]
async fn a_navigation_superseded_while_its_outlets_load_shows_nothing_of_them()
{
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/layout/page");
    settle().await;
    navigate("/other");
    settle().await;
    release("page");
    settle().await;
    assert!(!app.has("#layout") && !app.has("#page"));
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());

    release("other");
    settle().await;
    assert_eq!(app.text("#other").as_deref(), Some("loaded"));
    assert!(!app.has("#layout"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn replacing_the_child_of_a_loading_layout_shows_the_new_child_only() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/layout/page");
    settle().await;
    navigate("/layout/other");
    settle().await;
    release("page");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(!app.has("#layout") && !app.has("#page"));
    assert!(is_routing());

    release("other");
    settle().await;
    assert!(app.has("#layout"));
    assert_eq!(app.text("#other").as_deref(), Some("loaded"));
    assert!(!app.has("#page"), "the replaced child is shown");
    assert!(!is_routing());
}

/// A layout may render its child in more than one place, each `<Outlet/>`
/// with a view of its own.
#[wasm_bindgen_test]
async fn replacing_the_child_of_a_loading_layout_that_renders_it_twice() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/twice/page");
    settle().await;
    // the view of the first <Outlet/> loads, then the child is replaced
    release_first("page");
    settle().await;
    navigate("/twice/other");
    settle().await;
    release("page");
    settle().await;
    release("other");
    settle().await;
    release("other");
    settle().await;
    assert_eq!(app.text("#first #other").as_deref(), Some("loaded"));
    assert_eq!(app.text("#second #other").as_deref(), Some("loaded"));
    assert!(!app.has("#page"), "the replaced child is shown");
    assert!(!is_routing());
}

/// The new child is lazy, and has not loaded when the view of the one it
/// replaces has.
#[wasm_bindgen_test]
async fn replacing_the_child_of_a_loading_layout_with_a_lazy_route() {
    start_at("/");
    let app = mount(|| nested().into_any());
    settle().await;

    navigate("/layout/page");
    settle().await;
    navigate("/layout/lazy");
    settle().await;
    release("page");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(!app.has("#layout") && !app.has("#page"));
    assert!(is_routing());

    load_lazy_page().await;
    assert!(app.has("#layout") && app.has("#lazy"));
    assert!(!app.has("#page"), "the replaced child is shown");
    assert!(!is_routing());
}

async fn a_params_only_navigation_holds_is_routing_while_its_resources_reload(
    app: fn() -> AnyView,
) {
    start_at("/items/1");
    let app = mount(app);
    settle().await;
    release("item");
    settle().await;
    assert_eq!(app.text("#item").as_deref(), Some("item 1"));

    // the route is reused: only its params change, which reloads the item
    app.click("#to-item-2");
    settle().await;
    assert!(is_routing(), "the item is reloading");
    assert_eq!(app.text("#item").as_deref(), Some("item 1"));
    assert_eq!(pathname(), "/items/2", "the page itself is already shown");

    release("item");
    settle().await;
    assert_eq!(app.text("#item").as_deref(), Some("item 2"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_params_only_navigation_holds_is_routing_while_its_resources_reload()
 {
    a_params_only_navigation_holds_is_routing_while_its_resources_reload(
        || nested().into_any(),
    )
    .await;
}

#[wasm_bindgen_test]
async fn flat_params_only_navigation_holds_is_routing_while_its_resources_reload()
 {
    a_params_only_navigation_holds_is_routing_while_its_resources_reload(
        || flat().into_any(),
    )
    .await;
}

/// A params-only navigation to a route that an earlier navigation is still
/// loading holds the previous page, and is_routing, until that route has
/// loaded, and then while its resources reload for the new params.
async fn a_params_only_navigation_waits_for_the_route_it_reuses(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    navigate("/items/1");
    settle().await;
    navigate("/items/2");
    settle().await;
    assert!(app.has("#home"), "the previous page is held");
    assert!(is_routing());

    // the route loads for the first params, then reloads for the new ones
    release_first("item");
    settle().await;
    assert!(is_routing(), "the item is reloading for the new params");

    release("item");
    settle().await;
    assert_eq!(app.text("#item").as_deref(), Some("item 2"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn nested_params_only_navigation_waits_for_the_route_it_reuses() {
    a_params_only_navigation_waits_for_the_route_it_reuses(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_params_only_navigation_waits_for_the_route_it_reuses() {
    a_params_only_navigation_waits_for_the_route_it_reuses(|| {
        flat().into_any()
    })
    .await;
}

async fn a_navigation_away_from_a_reloading_route_completes(
    app: fn() -> AnyView,
) {
    start_at("/items/1");
    let app = mount(app);
    settle().await;
    release("item");
    settle().await;

    navigate("/items/2");
    settle().await;
    assert!(is_routing());
    navigate("/other");
    settle().await;
    release("other");
    settle().await;
    assert_eq!(app.text("#other").as_deref(), Some("loaded"));
    assert!(!is_routing(), "the abandoned reload holds the navigation");
}

#[wasm_bindgen_test]
async fn nested_navigation_away_from_a_reloading_route_completes() {
    a_navigation_away_from_a_reloading_route_completes(|| nested().into_any())
        .await;
}

#[wasm_bindgen_test]
async fn flat_navigation_away_from_a_reloading_route_completes() {
    a_navigation_away_from_a_reloading_route_completes(|| flat().into_any())
        .await;
}

/// A navigation that only changes the params of a route that an earlier
/// navigation is still loading completes once that route is on screen, with
/// or without `set_is_routing`.
async fn a_params_only_navigation_completes_with_the_route_it_reuses(
    app: fn() -> AnyView,
) {
    start_at("/");
    let app = mount(app);
    settle().await;

    app.click("#to-lazy-1");
    settle().await;
    app.click("#to-lazy-2");
    settle().await;
    assert!(app.has("#home"));
    assert_eq!(pathname(), "/", "the URL changes with the page");

    load_lazy_page().await;
    assert!(app.has("#lazy"));
    assert_eq!(pathname(), "/lazy/2");
}

#[wasm_bindgen_test]
async fn nested_params_only_navigation_completes_with_the_route_it_reuses() {
    a_params_only_navigation_completes_with_the_route_it_reuses(|| {
        nested().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_params_only_navigation_completes_with_the_route_it_reuses() {
    a_params_only_navigation_completes_with_the_route_it_reuses(|| {
        flat().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn nested_params_only_navigation_completes_with_the_route_it_reuses_without_set_is_routing()
 {
    a_params_only_navigation_completes_with_the_route_it_reuses(|| {
        nested_not_holding().into_any()
    })
    .await;
}

#[wasm_bindgen_test]
async fn flat_params_only_navigation_completes_with_the_route_it_reuses_without_set_is_routing()
 {
    a_params_only_navigation_completes_with_the_route_it_reuses(|| {
        flat_not_holding().into_any()
    })
    .await;
}

/// An app whose routes use view transitions, and which records, whenever
/// `is_routing` is cleared, whether the new page (`#page`) is on screen by the
/// time the browser can render again, i.e. once the microtasks queued with
/// that change have run.
fn with_view_transitions(flat: bool) -> AnyView {
    let (is_routing, set_is_routing) = signal(false);
    Effect::new(move |was_routing: Option<bool>| {
        let routing = is_routing.get();
        if was_routing == Some(true) && !routing {
            Executor::spawn_local(async {
                let mut shown = false;
                for _ in 0..8 {
                    shown =
                        document().query_selector("#page").unwrap().is_some();
                    if shown {
                        break;
                    }
                    Executor::tick().await;
                }
                PAGE_SHOWN_WHEN_ROUTED
                    .with(|seen| seen.borrow_mut().push(shown));
            });
        }
        routing
    });
    let fallback = || view! { <p id="not-found">"not found"</p> };
    let routes = move || {
        if flat {
            view! {
                <FlatRoutes transition=true fallback>
                    <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                    <Route path=path!("page") view=Page/>
                </FlatRoutes>
            }
            .into_any()
        } else {
            view! {
                <Routes transition=true fallback>
                    <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                    <Route path=path!("page") view=Page/>
                </Routes>
            }
            .into_any()
        }
    };
    view! {
        <Router set_is_routing>
            <CaptureNavigate/>
            {routes()}
        </Router>
    }
    .into_any()
}

/// Resolves once `done` holds, checking it every 10 milliseconds for a second.
async fn until(done: impl Fn() -> bool) {
    for _ in 0..100 {
        if done() {
            return;
        }
        sleep(10).await;
    }
    panic!("timed out");
}

/// With a view transition, the new page is shown once the browser has
/// captured the old one: the navigation completes only then.
async fn is_routing_is_cleared_once_a_view_transition_shows_the_page(
    flat: bool,
) {
    start_at("/");
    PAGE_SHOWN_WHEN_ROUTED.with(|seen| seen.borrow_mut().clear());
    let app = mount(move || with_view_transitions(flat));
    settle().await;

    navigate("/page");
    // <Routes> only creates the new view once the view transition has
    // captured the old page
    until(|| pending("page") > 0).await;
    release("page");
    until(|| app.has("#page")).await;
    settle().await;
    assert_eq!(app.text("#page").as_deref(), Some("loaded"));
    assert_eq!(
        PAGE_SHOWN_WHEN_ROUTED.with(|seen| seen.take()),
        [true],
        "is_routing was cleared before the new page was shown"
    );
}

#[wasm_bindgen_test]
async fn nested_is_routing_is_cleared_once_a_view_transition_shows_the_page() {
    is_routing_is_cleared_once_a_view_transition_shows_the_page(false).await;
}

#[wasm_bindgen_test]
async fn flat_is_routing_is_cleared_once_a_view_transition_shows_the_page() {
    is_routing_is_cleared_once_a_view_transition_shows_the_page(true).await;
}

/// An app whose `<Router>` has a main `<Routes>`, a side `<FlatRoutes>`, and a
/// `<Routes>` nested in a route of the main one.
fn several_routes() -> AnyView {
    view! {
        <Router set_is_routing=routing_setter()>
            <CaptureNavigate/>
            <Routes fallback=|| view! { <p id="not-found">"not found"</p> }>
                <Route path=path!("") view=|| view! { <p id="home">"home"</p> }/>
                <Route path=path!("both") view=Page/>
                <ParentRoute
                    path=path!("sub")
                    view=|| view! {
                        <Routes fallback=|| ()>
                            <Route path=path!("sub/a") view=|| view! { <p id="sub-a">"a"</p> }/>
                            <Route path=path!("sub/other") view=OtherPage/>
                        </Routes>
                    }
                >
                    <Route path=path!("a") view=|| ()/>
                    <Route path=path!("other") view=|| ()/>
                </ParentRoute>
            </Routes>
            <FlatRoutes fallback=|| ()>
                <Route path=path!("both") view=OtherPage/>
            </FlatRoutes>
        </Router>
    }
    .into_any()
}

#[wasm_bindgen_test]
async fn is_routing_stays_set_while_any_routes_of_the_router_navigate() {
    start_at("/");
    let app = mount(several_routes);
    settle().await;

    navigate("/both");
    settle().await;
    assert!(is_routing());
    release("other");
    settle().await;
    assert!(app.has("#other"));
    assert!(is_routing(), "the main <Routes> is still navigating");

    release("page");
    settle().await;
    assert!(app.has("#page"));
    assert!(!is_routing());
}

#[wasm_bindgen_test]
async fn routes_disposed_of_while_navigating_no_longer_hold_is_routing() {
    start_at("/sub/a");
    let app = mount(several_routes);
    settle().await;
    assert!(app.has("#sub-a"));

    navigate("/sub/other");
    settle().await;
    assert!(is_routing(), "the nested <Routes> is still navigating");

    // replaces the route that renders the nested <Routes>
    navigate("/");
    settle().await;
    assert!(app.has("#home"));
    assert!(!is_routing(), "the disposed <Routes> holds is_routing");
    release("other");
    settle().await;
    assert!(!is_routing());
}
