mod todo;

#[cfg(feature = "server")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use self::todo::server::*;
    use actix_web::*;

    let mut conn = db().await.expect("couldn't connect to DB");
    sqlx::migrate!()
        .run(&mut conn)
        .await
        .expect("could not run SQLx migrations");

    let addr = "127.0.0.1:3000";

    HttpServer::new(move || {
        App::new().route("/api/{tail:.*}", leptos_actix::handle_server_fns())
    })
    .bind(addr)?
    .run()
    .await
}

#[cfg(not(feature = "server"))]
pub fn main() {
    use crate::todo::*;
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    leptos::mount_to_body(TodoApp);
}
