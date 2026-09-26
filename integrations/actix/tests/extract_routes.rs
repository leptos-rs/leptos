//! Coverage for converting `leptos_router` paths into Actix path syntax via
//! `generate_route_list` / `to_actix_path`.
//!
//! Trailing-slash normalization is still undecided, so these cases avoid
//! trailing slashes and exercise only the stable conversions: static segments
//! pass through, `:param` becomes `{param}`, and `*splat` becomes
//! `{splat:.*}`.

use leptos::prelude::*;
use leptos_actix::generate_route_list;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "not found">
                <Route path=path!("/foo") view=|| ""/>
                <Route path=path!("/baz/:id") view=|| ""/>
                <Route path=path!("/baz/*any") view=|| ""/>
            </Routes>
        </Router>
    }
}

#[test]
fn converts_router_paths_to_actix_paths() {
    let routes = generate_route_list(App);

    let mut paths = routes
        .iter()
        .map(|r| r.path().to_string())
        .collect::<Vec<_>>();
    paths.sort();

    assert_eq!(paths, ["/baz/{any:.*}", "/baz/{id}", "/foo"]);
}
