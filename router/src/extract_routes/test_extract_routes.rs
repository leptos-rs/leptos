// This is here, vs /router/tests/, because it accesses some `pub(crate)`
// features to test crate internals that wouldn't be available there.

#![cfg(all(test, feature = "ssr"))]

use crate::*;
use itertools::Itertools;
use leptos::*;

#[component]
fn DefaultApp() -> impl IntoView {
    let view = || view! { "" };
    view! {
        <Router>
            <Routes>
                <Route path="/foo" view/>
                <Route path="/bar/" view/>
                <Route path="/baz/:id" view/>
                <Route path="/baz/:name/" view/>
                <Route path="/baz/*any" view/>
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
                <Route path="/baz/:name/" view/>
                <Route path="/baz/*any" view/>
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
                <Route path="/baz/:name/" view/>
                <Route path="/baz/*any" view/>
            </Routes>
        </Router>
    }
}
#[test]
fn test_generated_routes_default() {
    // By default, we use the behavior as of Leptos 0.5, which is equivalent to TrailingSlash::Drop.
    assert_generated_paths(
        DefaultApp,
        &["/bar", "/baz/*any", "/baz/:id", "/baz/:name", "/foo"],
    );
}

#[test]
fn test_generated_routes_exact() {
    // Allow users to precisely define whether slashes are present:
    assert_generated_paths(
        ExactApp,
        &["/bar/", "/baz/*any", "/baz/:id", "/baz/:name/", "/foo"],
    );
}

#[test]
fn test_generated_routes_redirect() {
    // TralingSlashes::Redirect generates paths to redirect to the path with the "correct" trailing slash ending (or lack thereof).
    assert_generated_paths(
        RedirectApp,
        &[
            "/bar",
            "/bar/",
            "/baz/*any",
            "/baz/:id",
            "/baz/:id/",
            "/baz/:name",
            "/baz/:name/",
            "/foo",
            "/foo/",
        ],
    )

    // TODO:
    // Test we get a redirect from "/foo/" to "/foo"
    // Test we get a redirect from "/bar" to "/bar/".
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
