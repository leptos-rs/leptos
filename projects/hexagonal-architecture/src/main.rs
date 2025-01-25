#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_hexagonal_design::{
        app::*,
        config::config,
        server_types::{HandlerStructAlias, ServerState},
    };

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);
    let handler = config();
    let handler_c = handler.clone();
    let server_state = ServerState {
        handler,
        leptos_options: leptos_options.clone(),
    };
    let app = Router::new()
        .leptos_routes_with_context(
            &server_state,
            routes,
            move || provide_context(handler_c.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<
            ServerState<HandlerStructAlias>,
            _,
        >(shell))
        .with_state(server_state);

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
