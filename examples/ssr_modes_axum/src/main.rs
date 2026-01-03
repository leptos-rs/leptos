#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        http::{HeaderName, HeaderValue},
        Router,
    };
    use leptos::{logging::log, prelude::*};
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use ssr_modes_axum::app::*;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler_with_context(
            move || {
                // if you want to add custom headers to the static file handler response,
                // you can do that by providing `ResponseOptions` via context
                let opts = use_context::<leptos_axum::ResponseOptions>()
                    .unwrap_or_default();
                opts.insert_header(
                    HeaderName::from_static("cross-origin-opener-policy"),
                    HeaderValue::from_static("same-origin"),
                );
                opts.insert_header(
                    HeaderName::from_static("cross-origin-embedder-policy"),
                    HeaderValue::from_static("require-corp"),
                );
                provide_context(opts);
            },
            shell,
        ))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
