mod todo;

#[cfg(feature = "ssr")]
mod ssr {
    pub use crate::todo::*;
    pub use actix_files::Files;
    pub use actix_web::*;
    pub use leptos::*;
    pub use leptos_actix::{generate_route_list, LeptosRoutes};

    #[get("/style.css")]
    pub async fn css() -> impl Responder {
        actix_files::NamedFile::open_async("./style.css").await
    }
}

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use self::{ssr::*, todo::ssr::*};

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
    let routes = generate_route_list(TodoApp);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        let routes = &routes;

        App::new()
            .service(css)
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                TodoApp,
            )
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(addr)?
    .run()
    .await
}
