use cfg_if::cfg_if;
mod todo;

// boilerplate to run in different modes
cfg_if! {
    // server-only stuff
    if #[cfg(feature = "ssr")] {
        use actix_files::{Files};
        use actix_web::*;
        use crate::todo::*;
        use leptos::*;
        use leptos_actix::LeptosRoutes;

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

            // Explicit server function registration is no longer required
            // on the main branch. On 0.3.0 and earlier, uncomment the lines
            // below to register the server functions.
            // _ = GetTodos::register();
            // _ = AddTodo::register();
            // _ = DeleteTodo::register();

            // Setting this to None means we'll be using cargo-leptos and its env vars.
            let conf = get_configuration(None).await.unwrap();
            let addr = conf.leptos_options.site_addr;

            HttpServer::new(move || {
                let leptos_options = &conf.leptos_options;
                let site_root = &leptos_options.site_root;

                App::new()
                    .service(css)
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .leptos_routes(leptos_options.to_owned(), TodoApp)
                    .service(Files::new("/", site_root))
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
