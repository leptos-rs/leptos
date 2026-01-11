#[cfg(feature = "ssr")]
mod router {
    use axum::Router;
    use clap::{Parser, Subcommand};
    use leptos::prelude::get_configuration;
    use leptos_axum::{ErrorHandler, LeptosRoutes, generate_route_list};
    use service_mode::app::{App, shell};

    #[derive(Parser)]
    pub struct Cli {
        #[command(subcommand)]
        mode: Mode,
    }

    #[derive(Subcommand)]
    enum Mode {
        Bare,
        Fallback,
        ErrorHandlerService,
        ErrorHandlerServiceFallback,
        RouteSitePkgNoFallback,
    }

    impl From<Cli> for Router {
        fn from(cli: Cli) -> Self {
            let conf = get_configuration(None).unwrap();
            let leptos_options = conf.leptos_options;
            let routes = generate_route_list(App);

            match cli.mode {
                Mode::Bare => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .with_state(leptos_options),
                Mode::Fallback => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .fallback(leptos_axum::file_and_error_handler(shell))
                    .with_state(leptos_options),
                Mode::ErrorHandlerService => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .fallback_service(ErrorHandler::new(
                        shell,
                        leptos_options.clone(),
                    ))
                    .with_state(leptos_options),
                Mode::ErrorHandlerServiceFallback => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .fallback_service(
                        leptos_axum::site_pkg_dir_service(&leptos_options)
                            .fallback(ErrorHandler::new(
                                shell,
                                leptos_options.clone(),
                            )),
                    )
                    .with_state(leptos_options),
                Mode::RouteSitePkgNoFallback => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .route_service(
                        &leptos_axum::site_pkg_dir_service_route_path(
                            &leptos_options,
                        ),
                        leptos_axum::site_pkg_dir_service(&leptos_options),
                    )
                    .fallback_service(ErrorHandler::new(
                        shell,
                        leptos_options.clone(),
                    ))
                    .with_state(leptos_options),
            }
        }
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use clap::Parser;
    use leptos::prelude::get_configuration;

    let app = Router::from(router::Cli::parse());
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
