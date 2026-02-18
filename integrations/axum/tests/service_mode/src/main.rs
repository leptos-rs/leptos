#[cfg(feature = "ssr")]
mod router {
    use axum::{
        Router,
        http::{HeaderName, HeaderValue},
    };
    use clap::{Parser, Subcommand};
    use leptos::prelude::{get_configuration, provide_context, use_context};
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
        FallbackWithContext,
        ErrorHandlerService,
        ErrorHandlerServiceFallback,
        RouteSitePkgNoFallback,
        RouteSitePkgDirMethod,
        RouteSitePkgDirFallbackMethod,

        ConfDefault,
        ConfDefaultWithSitePkg,
        ConfDefaultWithErrorHandler,
        ConfNew,
        ConfWithContext,
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
                Mode::FallbackWithContext => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .fallback(leptos_axum::file_and_error_handler_with_context(
                        move || {
                            let opts =
                                use_context::<leptos_axum::ResponseOptions>()
                                    .unwrap_or_default();
                            opts.insert_header(
                                HeaderName::from_static(
                                    "cross-origin-opener-policy",
                                ),
                                HeaderValue::from_static("same-origin"),
                            );
                            opts.insert_header(
                                HeaderName::from_static(
                                    "cross-origin-embedder-policy",
                                ),
                                HeaderValue::from_static("require-corp"),
                            );
                            provide_context(opts);
                        },
                        shell,
                    ))
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
                Mode::RouteSitePkgDirMethod => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    .leptos_route_site_pkg_dir(&leptos_options, shell)
                    .with_state(leptos_options),
                Mode::RouteSitePkgDirFallbackMethod => Router::new()
                    .leptos_routes(&leptos_options, routes, {
                        let leptos_options = leptos_options.clone();
                        move || shell(leptos_options.clone())
                    })
                    // to spice it up, different fallback "shells".
                    .leptos_route_site_pkg_dir(
                        &leptos_options,
                        |_| "site_pkg_dir fallback",
                    )
                    .leptos_route_fallback(&leptos_options, |_| "root fallback")
                    .with_state(leptos_options),

                Mode::ConfDefault => Router::new().leptos_route_configure(
                    leptos_axum::RouterConfiguration::default()
                        .app(App)
                        .shell(shell)
                        .state(leptos_options.clone()),
                ),
                Mode::ConfDefaultWithSitePkg => Router::new()
                    .leptos_route_configure(
                        leptos_axum::RouterConfiguration::default()
                            .app(App)
                            .shell(shell)
                            .state(leptos_options.clone())
                            .serve_site_pkg(true),
                    ),
                Mode::ConfDefaultWithErrorHandler => Router::new()
                    .leptos_route_configure(
                        leptos_axum::RouterConfiguration::default()
                            .app(App)
                            .shell(shell)
                            .state(leptos_options.clone())
                            .error_handler(true),
                    ),
                Mode::ConfNew => Router::new().leptos_route_configure(
                    leptos_axum::RouterConfiguration::new()
                        .app(App)
                        .shell(shell)
                        .state(leptos_options.clone()),
                ),
                Mode::ConfWithContext => Router::new().leptos_route_configure(
                    leptos_axum::RouterConfiguration::new()
                        .app(App)
                        .shell(shell)
                        .state(leptos_options.clone())
                        .with_context(move || {
                            let opts =
                                use_context::<leptos_axum::ResponseOptions>()
                                    .unwrap_or_default();
                            opts.insert_header(
                                HeaderName::from_static(
                                    "cross-origin-opener-policy",
                                ),
                                HeaderValue::from_static("same-origin"),
                            );
                            opts.insert_header(
                                HeaderName::from_static(
                                    "cross-origin-embedder-policy",
                                ),
                                HeaderValue::from_static("require-corp"),
                            );
                            provide_context(opts);
                        }),
                ),
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
    // write out the port from the bounded local_addr to allow the caller to know how to connect.
    println!("{}", listener.local_addr().unwrap().port());
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
