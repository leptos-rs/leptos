use leptos::{prelude::*, subsecond::connect_to_hot_patch_messages};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

fn main() {
    // connect to DX CLI and patch the WASM binary whenever we receive a message
    connect_to_hot_patch_messages();

    // wrapping App here in a closure so we can hot-reload it, because we only do that
    // for reactive views right now. changing anything will re-run App and update the view
    mount_to_body(|| App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <nav>
            <a href="/">"Home"</a>
            <a href="/about">"About"</a>
        </nav>
        <Router>
            <Routes fallback=|| "Not found">
                <Route path=path!("/") view=HomePage/>
                <Route path=path!("/about") view=About/>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Home Page"</h1>
    }
}

#[component]
fn About() -> impl IntoView {
    view! {
        <h1>"About"</h1>
    }
}
