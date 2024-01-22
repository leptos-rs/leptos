use leptos::*;
use leptos_meta::*;
use leptos_router::*;
mod api;
mod routes;
use routes::{nav::*, stories::*, story::*, users::*};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let (is_routing, set_is_routing) = create_signal(false);

    view! {
        <Stylesheet id="leptos" href="/pkg/hackernews.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        // adding `set_is_routing` causes the router to wait for async data to load on new pages
        <Router set_is_routing>
            // shows a progress bar while async data are loading
            <div class="routing-progress">
                <RoutingProgress is_routing max_time=std::time::Duration::from_millis(250)/>
            </div>
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
