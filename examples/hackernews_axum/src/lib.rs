use leptos::{component, view, IntoView};
use leptos_meta::*;
use leptos_router::*;
mod api;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fallback;
#[cfg(feature = "ssr")]
pub mod handlers;
mod routes;
use routes::{nav::*, stories::*, story::*, users::*};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/hackernews_axum.css"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Router>
            <Nav />
            <main>
                <Routes>
                    <Route path="users/:id" view=User/>
                    <Route path="stories/:id" view=Story/>
                    <Route path=":stories?" view=Stories/>
                </Routes>
            </main>
        </Router>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
