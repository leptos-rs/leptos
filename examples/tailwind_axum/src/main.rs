use axum::Router;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use leptos_meta::MetaTags;
use leptos_tailwind::app::App;
use leptos_tailwind::fallback::file_and_error_handler;

#[tokio::main]
async fn main() {
    // Setting this to None means we'll be using cargo-leptos and its env vars
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || {
                use leptos::prelude::*;

                view! {
                    <!DOCTYPE html>
                    <html lang="en">
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            // <AutoReload options=app_state.leptos_options.clone() />
                            <HydrationScripts options=leptos_options.clone()/>
                            <link rel="stylesheet" id="leptos" href="/pkg/benwis_leptos.css"/>
                            <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
                            <MetaTags/>
                        </head>
                        <body>
                            <App/>
                        </body>
                    </html>
                }
        }})
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
