#[cfg(feature = "ssr")]
mod latency {
    use std::sync::{Mutex, OnceLock};
    pub static LATENCY: OnceLock<
        Mutex<std::iter::Cycle<std::slice::Iter<'_, u64>>>,
    > = OnceLock::new();
    pub static ES_LATENCY: OnceLock<
        Mutex<std::iter::Cycle<std::slice::Iter<'_, u64>>>,
    > = OnceLock::new();
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        body::Body,
        extract::Request,
        http::{
            header::{self, HeaderValue},
            StatusCode,
        },
        middleware::{self, Next},
        response::{IntoResponse, Response},
        routing::get,
        Router,
    };
    use axum_js_ssr::app::*;
    use http_body_util::BodyExt;
    use leptos::{logging::log, prelude::*};
    use leptos_axum::{generate_route_list, LeptosRoutes};

    latency::LATENCY.get_or_init(|| [0, 4, 40, 400].iter().cycle().into());
    latency::ES_LATENCY.get_or_init(|| [0].iter().cycle().into());
    // Having the ES_LATENCY (a cycle of latency for the loading of the es
    // module) in an identical cycle as LATENCY (for the standard version)
    // adversely influences the intended demo, as this ultimately delays
    // hydration when set too high which can cause panic under every case.
    // If you want to test the effects of the delay just modify the list of
    // values for the desired cycle of delays.

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    async fn highlight_js() -> impl IntoResponse {
        (
            [(header::CONTENT_TYPE, "text/javascript")],
            include_str!(
                "../node_modules/@highlightjs/cdn-assets/highlight.min.js"
            ),
        )
    }

    async fn latency_for_highlight_js(
        req: Request,
        next: Next,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let uri_parts = &mut req.uri().path().rsplit('/');

        let is_highlightjs = uri_parts.next() == Some("highlight.min.js");
        let es = uri_parts.next() == Some("es");
        let module_type = if es { "es module " } else { "standard " };
        let res = next.run(req).await;
        if is_highlightjs {
            // additional processing if the filename is the test subject
            let (mut parts, body) = res.into_parts();
            let bytes = body
                .collect()
                .await
                .map_err(|err| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("error reading body: {err}"),
                    )
                })?
                .to_bytes();
            let latency = if es {
                &latency::ES_LATENCY
            } else {
                &latency::LATENCY
            };

            let delay = match latency
                .get()
                .expect("latency cycle wasn't set up")
                .try_lock()
            {
                Ok(ref mut mutex) => {
                    *mutex.next().expect("cycle always has next")
                }
                Err(_) => 0,
            };

            // inject the logging of the delay used into the target script
            log!(
                "loading {module_type}highlight.min.js with latency of \
                 {delay} ms"
            );
            let js_log = format!(
                "\nconsole.log('loaded {module_type}highlight.js with a \
                 minimum latency of {delay} ms');"
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

            let bytes = [bytes, js_log.into()].concat();
            let length = bytes.len();
            let body = Body::from(bytes);

            // Provide the bare minimum set of headers to avoid browser cache.
            parts.headers = header::HeaderMap::from_iter(
                [
                    (
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("text/javascript"),
                    ),
                    (header::CONTENT_LENGTH, HeaderValue::from(length)),
                ]
                .into_iter(),
            );
            Ok(Response::from_parts(parts, body))
        } else {
            Ok(res)
        }
    }

    let app = Router::new()
        .route("/highlight.min.js", get(highlight_js))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(middleware::from_fn(latency_for_highlight_js))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
