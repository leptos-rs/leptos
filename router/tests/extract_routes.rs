#![cfg(feature = "ssr")]

use itertools::Itertools;
use leptos::*;
use leptos_router::{Router, Routes, Route};

#[component]
fn ExampleApp() -> impl IntoView {
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

/// These routes are extracted and used by the actix/axum/viz integrations.
/// Make sure we're returning accurate paths to them. (w/ trailing slash).
#[test]
fn test_extracting_routes() {
    let (routes, hashmap) = leptos_router::generate_route_list_inner(ExampleApp);
    dbg!(hashmap.keys());

    let mut paths = routes
        .iter()
        .map(|route| route.path())
        .collect_vec();
    paths.sort();
    assert_eq!(paths, [
        "/bar/",
        "/baz/*any",
        "/baz/:id",
        "/baz/:name/",
        "/foo",
    ]);

    // integrations can update "path" to be valid for themselves, but
    // when routes are returned by leptos_router, these are equals:
    assert!(
        routes
            .iter()
            .all(|route| route.path() == route.leptos_path())
    );

    let mut keys = hashmap.keys().collect_vec();
    keys.sort();
    assert_eq!(paths, keys);

}

