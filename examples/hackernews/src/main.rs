use cfg_if::cfg_if;
use leptos::*;

// boilerplate to run in different modes
cfg_if! {
    // server-only stuff
    if #[cfg(feature = "ssr")] {
        use actix_files::{Files};
        use actix_web::*;
        use hackernews::{App,AppProps};

        #[get("/style.css")]
        async fn css() -> impl Responder {
            actix_files::NamedFile::open_async("./style.css").await
        }

        #[actix_web::main]
        async fn main() -> std::io::Result<()> {
            let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
            let addr = conf.leptos_options.site_address.clone();

            HttpServer::new(move || {
                let leptos_options = &conf.leptos_options;
                let site_root = &leptos_options.site_root;

                App::new()
                    .service(css)
                    .service(
                        web::scope("/{tail:.*}")
                        .guard(leptos_actix::HtmlGuard)
                        .route("", leptos_actix::render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <App/> }))
                    )
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .service(Files::new("/", &site_root)) // used by wasm-pack and cargo run. Can be removed if using cargo-leptos
                })
                .bind(&addr)?
                .run()
                .await
        }
    } else {
        fn main() {
            // no client-side main function
        }
    }
}
