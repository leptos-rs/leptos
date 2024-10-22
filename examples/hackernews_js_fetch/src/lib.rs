use leptos::prelude::*;
mod api;
mod routes;
use leptos_meta::{provide_meta_context, Link, Meta, MetaTags, Stylesheet};
use leptos_router::{
    components::{FlatRoutes, Route, Router, RoutingProgress},
    OptionalParamSegment, ParamSegment, StaticSegment,
};
use routes::{nav::*, stories::*, story::*, users::*};
use std::time::Duration;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
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
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Stylesheet id="leptos" href="/public/style.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/public/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Router set_is_routing>
            // shows a progress bar while async data are loading
            <div class="routing-progress">
                <RoutingProgress is_routing max_time=Duration::from_millis(250)/>
            </div>
            <Nav/>
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

#[cfg(feature = "ssr")]
mod ssr_imports {
    use crate::{shell, App};
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use log::{info, Level};
    use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    pub struct Handler(axum_js_fetch::App);

    #[wasm_bindgen]
    impl Handler {
        pub async fn new() -> Self {
            _ = console_log::init_with_level(Level::Debug);
            console_error_panic_hook::set_once();

            let leptos_options = LeptosOptions::builder()
                .output_name("client")
                .site_pkg_dir("pkg")
                .build();

            let routes = generate_route_list(App);

            // build our application with a route
            let app = Router::new()
                .leptos_routes(&leptos_options, routes, {
                    let leptos_options = leptos_options.clone();
                    move || shell(leptos_options.clone())
                })
                .with_state(leptos_options);

            info!("creating handler instance");

            Self(axum_js_fetch::App::new(app))
        }

        pub async fn serve(&self, req: web_sys::Request) -> web_sys::Response {
            self.0.oneshot(req).await
        }
    }
}
