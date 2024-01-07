use leptos::*;
use leptos_router::{Router, Routes, Route};
use leptos_actix::generate_route_list;

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

#[test]
fn test_extracting_routes() {
    let routes = generate_route_list(ExampleApp);

    // We still have access to the original Leptos paths:
    let mut leptos_paths: Vec<_> = routes
        .iter()
        .map(|route| route.leptos_path())
        .collect();
    leptos_paths.sort();
    assert_eq!(leptos_paths, [
        "/bar/",
        "/baz/*any",
        "/baz/:id",
        "/baz/:name/",
        "/foo",
    ]);

    // ... But leptos-actix has also reformatted "paths" to work for Actix.
    let mut paths: Vec<_> = routes
        .iter()
        .map(|route| route.path())
        .collect();
    paths.sort();
    assert_eq!(paths, [
        "/bar/",
        "/baz/{id}",
        "/baz/{name}/",
        "/baz/{tail:.*}",
        "/foo",
    ]);
}

