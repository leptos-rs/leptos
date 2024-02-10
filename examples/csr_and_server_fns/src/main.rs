mod todo;

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use self::{ssr::*, todo::ssr::*};

    let mut conn = db().await.expect("couldn't connect to DB");
    sqlx::migrate!()
        .run(&mut conn)
        .await
        .expect("could not run SQLx migrations");

    // Setting this to None means we'll be using cargo-leptos and its env vars.
    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();

    let addr = conf.leptos_options.site_addr;
    println!("Server functions available at http://{}", &addr);

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(TodoApp);
    println!("routes: {:?}", routes);

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
    })
    .bind(addr)?
    .run()
    .await
}
 
#[cfg(feature = "csr")]
pub fn main() {
    use crate::todo::*;
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount_to_body(TodoApp);
}