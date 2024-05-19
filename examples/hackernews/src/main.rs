// server-only stuff
#[cfg(feature = "ssr")]
mod ssr_imports {
    pub use actix_files::Files;
    pub use actix_web::*;
    pub use hackernews::App;
    pub use leptos_actix::{generate_route_list, LeptosRoutes};

    #[get("/style.css")]
    pub async fn css() -> impl Responder {
        actix_files::NamedFile::open_async("./style.css").await
    }
    #[get("/favicon.ico")]
    pub async fn favicon() -> impl Responder {
        actix_files::NamedFile::open_async("./target/site//favicon.ico").await
    }
}

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use leptos::get_configuration;
    use ssr_imports::*;

    // Setting this to None means we'll be using cargo-leptos and its env vars.
    let conf = get_configuration(None).await.unwrap();

    let addr = conf.leptos_options.site_addr;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .service(css)
            .service(favicon)
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

// CSR-only setup
#[cfg(not(feature = "ssr"))]
fn main() {
    use hackernews::App;

    use tracing_subscriber::fmt;
    use tracing_subscriber_wasm::MakeConsoleWriter;

    fmt()
        .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            MakeConsoleWriter::default()
                .map_trace_level_to(tracing::Level::DEBUG),
        )
        // For some reason, if we don't do this in the browser, we get
        // a runtime error.
        .without_time()
        .init();
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App)
}
