// server-only stuff
#[cfg(feature = "ssr")]
mod ssr_imports {
    pub use actix_files::Files;
    pub use actix_web::*;
    pub use hackernews::App;

    #[get("/style.css")]
    pub async fn css() -> impl Responder {
        actix_files::NamedFile::open_async("./style.css").await
    }
    #[get("/favicon.ico")]
    pub async fn favicon() -> impl Responder {
        actix_files::NamedFile::open_async("./target/site//favicon.ico").await
    }
}

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_meta::MetaTags;
    use ssr_imports::*;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    HttpServer::new(move || {
        // Generate the list of routes in your Leptos App
        let routes = generate_route_list(App);
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .service(css)
            .service(favicon)
            .leptos_routes(routes, {
                let leptos_options = leptos_options.clone();
                move || {
                    use leptos::prelude::*;

                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body>
                                <App/>
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

// CSR-only setup
#[cfg(not(feature = "ssr"))]
fn main() {
    use hackernews::App;
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App)
}
