use leptos::prelude::*;
mod api;
mod routes;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet};
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    OptionalParamSegment, ParamSegment, StaticSegment,
};
use routes::{nav::*, stories::*, story::*, users::*};
#[cfg(feature = "ssr")]
pub mod fallback;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options islands=true/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/hackernews.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Router>
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
    leptos::mount::hydrate_islands();
}
