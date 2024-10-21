mod todo;

#[cfg(feature = "ssr")]
mod ssr {
    pub use crate::todo::*;
    pub use actix_files::Files;
    pub use actix_web::*;
    pub use leptos::prelude::*;
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

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        // Generate the list of routes in your Leptos App
        let routes = generate_route_list(TodoApp);
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .leptos_routes(routes, {
                let leptos_options = leptos_options.clone();
                move || {
                    use leptos::prelude::*;

                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta
                                    name="viewport"
                                    content="width=device-width, initial-scale=1"
                                />
                                <AutoReload options=leptos_options.clone()/>
                                <HydrationScripts options=leptos_options.clone()/>
                            </head>
                            <body>
                                <TodoApp/>
                            </body>
                        </html>
                    }
            }})
            .service(Files::new("/", site_root.as_ref()))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}
