#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use futures::StreamExt;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes, render_app_to_stream, render_app_async, render_app_to_stream_in_order};
    use leptos_meta::provide_meta_context;
    use leptos_start::app::*;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(|cx| view! { cx, <App/> });

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            .route("/", render_app_to_stream_in_order(leptos_options.to_owned(), |cx| view! { cx, <App/> }))
            .route("/ooo", render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <App/> }))
            .route("/async", render_app_async(leptos_options.to_owned(), |cx| view! { cx, <App/> }))
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
