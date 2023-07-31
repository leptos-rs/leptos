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
        use leptos_actix::{generate_route_list, LeptosRoutes};

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

            // Generate the list of routes in your Leptos App
            let routes = generate_route_list(|cx| view! { cx, <TodoApp/> });

            HttpServer::new(move || {
                let leptos_options = &conf.leptos_options;
                let site_root = &leptos_options.site_root;
                let routes = &routes;

                App::new()
                    .service(css)
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .service(Files::new("/pkg", format!("{site_root}/pkg")))
                    .service(Files::new("/assets", site_root))
                    .service(favicon)
                    .leptos_routes(
                        leptos_options.to_owned(),
                        routes.to_owned(),
                        TodoApp,
                    )
                    .app_data(web::Data::new(leptos_options.to_owned()))
                    //.wrap(middleware::Compress::default())
            })
            .bind(addr)?
            .run()
            .await
        }

        #[actix_web::get("favicon.ico")]
        async fn favicon(
            leptos_options: actix_web::web::Data<leptos::LeptosOptions>,
        ) -> actix_web::Result<actix_files::NamedFile> {
            let leptos_options = leptos_options.into_inner();
            let site_root = &leptos_options.site_root;
            Ok(actix_files::NamedFile::open(format!(
                "{site_root}/favicon.ico"
            ))?)
        }
    } else {
        fn main() {
            // no client-side main function
        }
    }
}
