#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_files::Files;
    use actix_web::*;
    use futures::StreamExt;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_meta::provide_meta_context;
    use leptos_start::app::*;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let addr = conf.leptos_options.site_address;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(|cx| view! { cx, <App/> });

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            .route(
                "/",
                web::get().to({
                    let leptos_options = leptos_options.clone();
                    move |req: actix_web::HttpRequest| {
                        let leptos_options = leptos_options.clone();
                        async move {
                            let site_root = &leptos_options.site_root;
                            let pkg_path = leptos_options.site_pkg_dir.clone();
                            let output_name = leptos_options.output_name.clone();
                            let site_ip = leptos_options.site_address.ip().to_string();
                            let reload_port = leptos_options.reload_port;

                            let mut wasm_output_name = output_name.clone();
                            if std::env::var("LEPTOS_OUTPUT_NAME").is_err() {
                                wasm_output_name.push_str("_bg");
                            }

                                                                let leptos_autoreload = match std::env::var("LEPTOS_WATCH").is_ok() {
                                            true => format!(
                                                r#"
                                                <script crossorigin="">(function () {{
                                                    var ws = new WebSocket('ws://{site_ip}:{reload_port}/live_reload');
                                                    ws.onmessage = (ev) => {{
                                                        let msg = JSON.parse(ev.data);
                                                        if (msg.all) window.location.reload();
                                                        if (msg.css) {{
                                                            const link = document.querySelector("link#leptos");
                                                            if (link) {{
                                                                let href = link.getAttribute('href').split('?')[0];
                                                                let newHref = href + '?version=' + new Date().getMilliseconds();
                                                                link.setAttribute('href', newHref);
                                                            }} else {{
                                                                console.warn("Could not find link#leptos");
                                                            }}
                                                        }};
                                                    }};
                                                    ws.onclose = () => console.warn('Live-reload stopped. Manual reload necessary.');
                                                }})()
                                                </script>
                                                "#
                                            ),
                                            false => "".to_string(),
                                        };

                            let res_options = leptos_actix::ResponseOptions::default();

                            let (stream, runtime, scope) =
                            render_to_stream_in_order_undisposed_with_prefix_and_suffix_and_context(
                                |cx| view! { cx, <App/> }.into_view(cx),
                                {
                                    let pkg_path = pkg_path.clone();
                                    let output_name = output_name.clone();
                                    let wasm_output_name = wasm_output_name.clone();
                                    move |cx| {
                                    let meta = use_context::<leptos_meta::MetaContext>(cx);
                                    let html_metadata =
                                        meta.as_ref().and_then(|mc| mc.html.as_string()).unwrap_or_default();
                                    let head = meta
                                        .as_ref()
                                        .map(|meta| meta.dehydrate())
                                        .unwrap_or_default();
                                    let body_meta = meta
                                        .as_ref()
                                        .and_then(|meta| meta.body.as_string())
                                        .unwrap_or_default();
                                    format!(r#"
                                        <!DOCTYPE html>
                                        <html{html_metadata}>
                                            <head>
                                            <link rel="modulepreload" href="/{pkg_path}/{output_name}.js">
                                            <link rel="preload" href="/{pkg_path}/{wasm_output_name}.wasm" as="fetch" type="application/wasm" crossorigin="">
                                            {head}
                                            </head>
                                            <body{body_meta}>"#).into()
                                }},
                                move |_| {
                                    format!(r#"
                                    <script type="module">import init, {{ hydrate }} from '/{pkg_path}/{output_name}.js'; init('/{pkg_path}/{wasm_output_name}.wasm').then(hydrate);</script>
                                    {leptos_autoreload}
                                    "#).into()
                                },
                                move |cx| {
                                    fn provide_contexts(
                                        cx: leptos::Scope,
                                        req: &HttpRequest,
                                        res_options: leptos_actix::ResponseOptions,
                                    ) {
                                        let path = leptos_corrected_path(req);

                                        let integration = leptos_router::ServerIntegration { path };
                                        provide_context(
                                            cx,
                                            leptos_router::RouterIntegrationContext::new(
                                                integration,
                                            ),
                                        );
                                        provide_context(cx, leptos_meta::MetaContext::new());
                                        provide_context(cx, res_options);
                                        provide_context(cx, req.clone());
                                    }

                                    fn leptos_corrected_path(req: &HttpRequest) -> String {
                                        let path = req.path();
                                        let query = req.query_string();
                                        if query.is_empty() {
                                            "http://leptos".to_string() + path
                                        } else {
                                            "http://leptos".to_string() + path + "?" + query
                                        }
                                    }

                                    provide_contexts(cx, &req, res_options)
                                },
                            );
                            let stream =
                                stream.map(|html| Ok(web::Bytes::from(html)) as Result<web::Bytes>);

                            HttpResponse::Ok()
                                .content_type("text/html")
                                .streaming(stream)
                        }
                    }
                }),
            )
            /* .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                |cx| view! { cx, <App/> },
            ) */
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
