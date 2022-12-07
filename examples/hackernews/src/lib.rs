use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
mod api;
mod routes;
use routes::nav::*;
use routes::stories::*;
use routes::story::*;
use routes::users::*;

#[component]
pub fn App(cx: Scope) -> Element {
    provide_context(cx, MetaContext::default());

    view! {
        cx,
        <div>
            <Stylesheet href="/style.css"/>
            <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
            <Router>
                <Nav />
                <main>
                    <Routes>
                        <Route path="users/:id" element=|cx| view! { cx,  <User/> }/>
                        <Route path="stories/:id" element=|cx| view! { cx,  <Story/> }/>
                        <Route path="*stories" element=|cx| view! { cx,  <Stories/> }/>
                    </Routes>
                </main>
            </Router>
        </div>
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
            leptos::hydrate(body().unwrap(), move |cx| {
                view! { cx, <App/> }
            });
        }
    }
}
