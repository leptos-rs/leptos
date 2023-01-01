use cfg_if::cfg_if;
use leptos::*;
mod todo;

// boilerplate to run in different modes
cfg_if! {
    // server-only stuff
    if #[cfg(feature = "ssr")] {
        use actix_files::{Files};
        use actix_web::*;
        use crate::todo::*;

        #[get("/style.css")]
        async fn css() -> impl Responder {
            actix_files::NamedFile::open_async("./style.css").await
        }

        #[actix_web::main]
        async fn main() -> std::io::Result<()> {
            let mut conn = db().await.expect("couldn't connect to DB");
            sqlx::migrate!()
                .run(&mut conn)
                .await
                .expect("could not run SQLx migrations");

            crate::todo::register_server_functions();


            let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
            let addr = conf.leptos_options.site_address.clone();

            HttpServer::new(move || {
                let leptos_options = &conf.leptos_options;
                let site_root = &leptos_options.site_root;
                let pkg_dir = &leptos_options.site_pkg_dir;
                let bundle_path = format!("/{site_root}/{pkg_dir}");

                App::new()
                    .service(Files::new("/pkg", "./pkg")) // used by wasm-pack and cargo run. Can be removed if using cargo-leptos
                    .service(Files::new(&bundle_path, format!("./{bundle_path}"))) // used by cargo-leptos. Can be removed if using wasm-pack and cargo run.
                    .service(css)
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .route("/{tail:.*}", leptos_actix::render_app_to_stream(leptos_options.to_owned(), |cx| view! { cx, <TodoApp/> }))
                //.wrap(middleware::Compress::default())
            })
            .bind(addr)?
            .run()
            .await
        }
    } else {
        fn main() {
            // no client-side main function
        }
    }
}
