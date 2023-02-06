use cfg_if::cfg_if;
use leptos::{component, view, IntoView, Scope};
use leptos_meta::*;
use leptos_router::*;
mod api;
mod routes;
use routes::nav::*;
use routes::stories::*;
use routes::story::*;
use routes::users::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    provide_meta_context(cx);
    view! {
        cx,
        <>
            <Stylesheet id="leptos" href="/pkg/hackernews.css"/>
            <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
            <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
            <Router>
                <Nav />
                <main>
                    <Routes>
                        <Route path="users/:id" view=|cx| view! { cx,  <User/> }/>
                        <Route path="stories/:id" view=|cx| view! { cx,  <Story/> }/>
                        <Route path=":stories?" view=|cx| view! { cx,  <Stories/> }/>
                    </Routes>
                </main>
            </Router>
        </>
    }
}

// Needs to be in lib.rs AFAIK because wasm-bindgen needs us to be compiling a lib. I may be wrong.
cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use wasm_bindgen::prelude::wasm_bindgen;

        #[wasm_bindgen]
        pub fn hydrate() {
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            leptos::mount_to_body(move |cx| {
                view! { cx, <App/> }
            });
        }
    }
}
