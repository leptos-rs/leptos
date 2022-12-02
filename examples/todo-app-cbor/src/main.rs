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

            HttpServer::new(|| {
                App::new()
                    .service(Files::new("/pkg", "./pkg"))
                    .service(css)
                    .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
                    .route("/{tail:.*}", leptos_actix::render_app_to_stream("/pkg/todo_app_cbor", |cx| view! { cx, <TodoApp/> }))
                //.wrap(middleware::Compress::default())
            })
            .bind(("127.0.0.1", 8081))?
            .run()
            .await
        }
    } else {
        fn main() {
            // no client-side main function
        }
    }
}
