// This is here, vs /router/tests/, because it accesses some `pub(crate)`
// features to test crate internals that wouldn't be available there.

#![cfg(all(test, feature = "ssr"))]

use crate::*;
use itertools::Itertools;
use leptos::*;
use std::{cell::RefCell, rc::Rc};

#[component]
fn DefaultApp() -> impl IntoView {
    let view = || view! { "" };
    view! {
        <Router>
            <Routes>
                <Route path="/foo" view/>
                <Route path="/bar/" view/>
                <Route path="/baz/:id" view/>
                <Route path="/name/:name/" view/>
                <Route path="/any/*any" view/>
            </Routes>
        </Router>
    }
}

#[component]
fn ExactApp() -> impl IntoView {
    let view = || view! { "" };
    let trailing_slash = TrailingSlash::Exact;
    view! {
        <Router trailing_slash>
            <Routes>
                <Route path="/foo" view/>
                <Route path="/bar/" view/>
                <Route path="/baz/:id" view/>
                <Route path="/name/:name/" view/>
                <Route path="/any/*any" view/>
            </Routes>
        </Router>
    }
}

#[component]
fn RedirectApp() -> impl IntoView {
    let view = || view! { "" };
    let trailing_slash = TrailingSlash::Redirect;
    view! {
        <Router trailing_slash>
            <Routes>
                <Route path="/foo" view/>
                <Route path="/bar/" view/>
                <Route path="/baz/:id" view/>
                <Route path="/name/:name/" view/>
                <Route path="/any/*any" view/>
            </Routes>
        </Router>
    }
}
#[test]
fn test_generated_routes_default() {
    // By default, we use the behavior as of Leptos 0.5, which is equivalent to TrailingSlash::Drop.
    assert_generated_paths(
        DefaultApp,
        &["/any/*any", "/bar", "/baz/:id", "/foo", "/name/:name"],
    );
}

#[test]
fn test_generated_routes_exact() {
    // Allow users to precisely define whether slashes are present:
    assert_generated_paths(
        ExactApp,
        &["/any/*any", "/bar/", "/baz/:id", "/foo", "/name/:name/"],
    );
}

#[test]
fn test_generated_routes_redirect() {
    // TralingSlashes::Redirect generates paths to redirect to the path with the "correct" trailing slash ending (or lack thereof).
    assert_generated_paths(
        RedirectApp,
        &[
            "/any/*any",
            "/bar",
            "/bar/",
            "/baz/:id",
            "/baz/:id/",
            "/foo",
            "/foo/",
            "/name/:name",
            "/name/:name/",
        ],
    )
}

#[test]
fn test_rendered_redirect() {
    // Given an app that uses TrailngSlsahes::Redirect, rendering the redirected path
    // should render the redirect. Other paths should not.

    let expected_redirects = &[
        ("/bar", "/bar/"),
        ("/baz/some_id/", "/baz/some_id"),
        ("/name/some_name", "/name/some_name/"),
        ("/foo/", "/foo"),
    ];

    let redirect_result = Rc::new(RefCell::new(Option::None));
    let rc = redirect_result.clone();
    let server_redirect = move |new_value: &str| {
        rc.replace(Some(new_value.to_string()));
    };

    let _runtime = Disposable(create_runtime());
    let history = TestHistory::new("/");
    provide_context(RouterIntegrationContext::new(history.clone()));
    provide_server_redirect(server_redirect);

    // We expect these redirects to exist:
    for (src, dest) in expected_redirects {
        let loc = format!("https://example.com{src}");
        history.goto(&loc);
        redirect_result.replace(None);
        RedirectApp().into_view().render_to_string();
        let redirected_to = redirect_result.borrow().clone();
        assert!(
            redirected_to.is_some(),
            "Should redirect from {src} to {dest}"
        );
        assert_eq!(redirected_to.unwrap(), *dest);
    }

    // But the destination paths shouldn't themselves redirect:
    redirect_result.replace(None);
    for (_src, dest) in expected_redirects {
        let loc = format!("https://example.com{dest}");
        history.goto(&loc);
        RedirectApp().into_view().render_to_string();
        let redirected_to = redirect_result.borrow().clone();
        assert!(
            redirected_to.is_none(),
            "Destination of redirect shouldn't also redirect: {dest}"
        );
    }
}

struct Disposable(RuntimeId);

// If the test fails, and we don't dispose, we get irrelevant panics.
impl Drop for Disposable {
    fn drop(&mut self) {
        self.0.dispose()
    }
}

#[derive(Clone)]
struct TestHistory {
    loc: RwSignal<LocationChange>,
}

impl TestHistory {
    fn new(initial: &str) -> Self {
        let lc = LocationChange {
            value: initial.to_owned(),
            ..Default::default()
        };
        Self {
            loc: create_rw_signal(lc),
        }
    }

    fn goto(&self, loc: &str) {
        let change = LocationChange {
            value: loc.to_string(),
            ..Default::default()
        };

        self.navigate(&change);
    }
}

impl History for TestHistory {
    fn location(&self) -> ReadSignal<LocationChange> {
        self.loc.read_only()
    }

    fn navigate(&self, new_loc: &LocationChange) {
        self.loc.update(|loc| loc.value.clone_from(&new_loc.value))
    }
}

// WARNING!
//
// Despite generate_route_list_inner() using a new leptos_reactive::RuntimeID
// each time we call this function, somehow Routes are leaked between different
// apps. To avoid that, make sure to put each call in a separate #[test] method.
//
// TODO: Better isolation for different apps to avoid this issue?
fn assert_generated_paths<F, IV>(app: F, expected_sorted_paths: &[&str])
where
    F: Clone + Fn() -> IV + 'static,
    IV: IntoView + 'static,
{
    let (routes, static_data) = generate_route_list_inner(app);

    let mut paths = routes.iter().map(|route| route.path()).collect_vec();
    paths.sort();

    assert_eq!(paths, expected_sorted_paths);

    let mut keys = static_data.keys().collect_vec();
    keys.sort();
    assert_eq!(paths, keys);

    // integrations can update "path" to be valid for themselves, but
    // when routes are returned by leptos_router, these are equal:
    assert!(routes
        .iter()
        .all(|route| route.path() == route.leptos_path()));
}

#[test]
fn test_unique_route_ids() {
    let branches = get_branches(RedirectApp);
    assert!(!branches.is_empty());

    assert!(branches
        .iter()
        .flat_map(|branch| &branch.routes)
        .map(|route| route.id)
        .all_unique());
}

#[test]
fn test_unique_route_patterns() {
    let branches = get_branches(RedirectApp);
    assert!(!branches.is_empty());

    assert!(branches
        .iter()
        .flat_map(|branch| &branch.routes)
        .map(|route| route.pattern.as_str())
        .all_unique());
}

fn get_branches<F, IV>(app_fn: F) -> Vec<Branch>
where
    F: Fn() -> IV + Clone + 'static,
    IV: IntoView + 'static,
{
    let runtime = create_runtime();
    let additional_context = || ();
    let branches = super::get_branches(app_fn, additional_context);
    let branches = branches.0.borrow().clone();
    runtime.dispose();
    branches
}
