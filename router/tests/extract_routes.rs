#![cfg(feature = "ssr")]

use itertools::Itertools;
use leptos::*;
use leptos_router::{
    generate_route_list_inner, Branch, Route, Router, Routes, TrailingSlash,
};

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
            "/baz/*any/", // !!! TODO
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

/// This is how [`generate_route_list_inner`] gets its RouteDefinitions.
/// But it doesn't expose it anywhere where we can easily test it, so here's a quick copy:
fn get_branches<F, IV>(app_fn: F) -> Vec<Branch>
where
    F: Fn() -> IV + Clone + 'static,
    IV: IntoView,
{
    use leptos_router::*;

    let runtime = create_runtime();

    let integration = ServerIntegration {
        path: "http://leptos.rs/".to_string(),
    };

    provide_context(RouterIntegrationContext::new(integration));
    let branches = PossibleBranchContext::default();
    provide_context(branches.clone());

    leptos::suppress_resource_load(true);
    _ = app_fn().into_view();
    leptos::suppress_resource_load(false);

    let branches = branches.0.borrow().clone();
    runtime.dispose();
    branches
}
