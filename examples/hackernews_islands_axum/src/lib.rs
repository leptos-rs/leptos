#![feature(lazy_cell)]

use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
mod api;
mod routes;
use routes::{nav::*, stories::*, story::*, users::*};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/style.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Router>
            <Nav />
            <main>
                <Routes>
                    <Route path="users/:id" view=User ssr=SsrMode::InOrder/>
                    <Route path="stories/:id" view=Story ssr=SsrMode::InOrder/>
                    <Route path=":stories?" view=Stories ssr=SsrMode::InOrder/>
                </Routes>
            </main>
        </Router>
    }
}

// Needs to be in lib.rs AFAIK because wasm-bindgen needs us to be compiling a lib. I may be wrong.
cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use wasm_bindgen::prelude::wasm_bindgen;

        extern crate wee_alloc;

        // Use `wee_alloc` as the global allocator.
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

        #[wasm_bindgen]
        pub fn hydrate() {
            #[cfg(debug_assertions)]
            console_error_panic_hook::set_once();
            leptos::leptos_dom::HydrationCtx::stop_hydrating();
        }
    }
}
