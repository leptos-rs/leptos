mod app;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use actix_files::Files;
        use actix_web::*;
        use leptos::*;
        use crate::app::*;

        #[get("/style.css")]
        async fn css() -> impl Responder {
            actix_files::NamedFile::open_async("./style/output.css").await
        }

        #[actix_web::main]
        async fn main() -> std::io::Result<()> {
            let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
            let addr = conf.leptos_options.site_address.clone();
            let leptos_options = &conf.leptos_options;
            let site_root = &leptos_options.site_root;
            let pkg_dir = &leptos_options.site_pkg_dir;
            let bundle_path = format!("/{site_root}/{pkg_dir}");

            HttpServer::new(move || {
                let leptos_options = &conf.leptos_options;
                App::new()
                    .service(css)
                    .service(Files::new("/pkg", "./pkg")) // used by wasm-pack and cargo run. Can be removed if using cargo-leptos
                    .service(Files::new(&bundle_path, format!("./{bundle_path}"))) // used by cargo-leptos. Can be removed if using wasm-pack and cargo run.
                    .route("/{tail:.*}", leptos_actix::render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <App/> }))
                    .wrap(middleware::Compress::default())
            })
            .bind(&addr)?
            .run()
            .await
        }
    }
    else {
        pub fn main() {}
    }
}
