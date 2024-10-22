use leptos::prelude::*;
mod api;
mod routes;
use leptos_meta::{provide_meta_context, Link, Meta, Stylesheet};
use leptos_router::{
    components::{FlatRoutes, Route, Router, RoutingProgress},
    OptionalParamSegment, ParamSegment, StaticSegment,
};
use routes::{nav::*, stories::*, story::*, users::*};
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Stylesheet id="leptos" href="/pkg/hackernews.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Router set_is_routing>
            // shows a progress bar while async data are loading
            <div class="routing-progress">
                <RoutingProgress is_routing max_time=Duration::from_millis(250)/>
            </div>
            <Nav />
            <main>
                <FlatRoutes fallback=|| "Not found.">
                    <Route path=(StaticSegment("users"), ParamSegment("id")) view=User/>
                    <Route path=(StaticSegment("stories"), ParamSegment("id")) view=Story/>
                    <Route path=OptionalParamSegment("stories") view=Stories/>
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
