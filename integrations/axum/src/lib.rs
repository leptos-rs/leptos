use axum::{
    body::{Body, Bytes, StreamBody},
    http::Request,
};
use futures::{Future, SinkExt, Stream, StreamExt};
use leptos::*;
use leptos_meta::MetaContext;
use leptos_router::*;
use std::io;
use std::pin::Pin;

pub type PinnedHtmlStream = Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>;

/// Returns an Axum [Handler](axum::Handler) that listens for a `GET` request and tries
/// to route it using [leptos_router], serving an HTML stream of your application.
///
/// The provides a [MetaContext] and a [RouterIntegrationContext] to app’s context before
/// rendering it, and includes any meta tags injected using [leptos_meta].
///
/// The HTML stream is rendered using [render_to_stream], and includes everything described in
/// the documentation for that function.
///
/// This can then be set up at an appropriate route in your application:
/// ```
/// use axum::Router;
/// use std::net::SocketAddr;
/// use leptos::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> Element {
///   view! { cx, <main>"Hello, world!"</main> }
/// }
///
/// # if false { // don't actually try to run a server in a doctest...
/// #[tokio::main]
/// async fn main() {
///     let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
///
///     // build our application with a route
///     let app = Router::new()
///     .fallback(leptos_axum::render_app_to_stream("leptos_example", |cx| view! { cx, <MyApp/> }).into_service());
///
///     // run our app with hyper
///     // `axum::Server` is a re-export of `hyper::Server`
///     axum::Server::bind(&addr)
///         .serve(app.into_make_service())
///         .await
///         .unwrap();
/// }
/// # }
/// ```
pub fn render_app_to_stream(
    client_pkg_name: &'static str,
    app_fn: impl Fn(leptos::Scope) -> Element + Clone + Send + 'static,
) -> impl Fn(Request<Body>) -> Pin<Box<dyn Future<Output = StreamBody<PinnedHtmlStream>>>>
       + Clone
       + Send
       + 'static {
    move |req: Request<Body>| {
        Box::pin({
            let app_fn = app_fn.clone();
            async move {
                // Need to get the path and query string of the Request
                let path = req.uri();
                let query = path.query();

                let full_path;
                if let Some(query) = query {
                    full_path = "http://leptos".to_string() + &path.to_string() + "?" + query
                } else {
                    full_path = "http://leptos".to_string() + &path.to_string()
                }

                let head = format!(
                    r#"<!DOCTYPE html>
                    <html lang="en">
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            <script type="module">import init, {{ hydrate }} from '/pkg/{client_pkg_name}.js'; init().then(hydrate);</script>"#
                );
                let tail = "</body></html>";

                let (mut tx, rx) = futures::channel::mpsc::channel(8);

                std::thread::spawn({
                    let app_fn = app_fn.clone();
                    move || {
                        tokio::runtime::Runtime::new()
                            .expect("couldn't spawn runtime")
                            .block_on({
                                let app_fn = app_fn.clone();
                                async move {
                                    tokio::task::LocalSet::new()
                                        .run_until(async {
                                            let mut shell = Box::pin(render_to_stream({
                                                let full_path = full_path.clone();
                                                move |cx| {
                                                    let app = {
                                                        let full_path = full_path.clone();
                                                        let app_fn = app_fn.clone();
                                                        move |cx| {
                                                            let integration = ServerIntegration {
                                                                path: full_path.clone(),
                                                            };
                                                            provide_context(
                                                                cx,
                                                                RouterIntegrationContext::new(
                                                                    integration,
                                                                ),
                                                            );
                                                            provide_context(cx, MetaContext::new());

                                                            (app_fn)(cx)
                                                        }
                                                    };
                                                    let app = app(cx);
                                                    let head = use_context::<MetaContext>(cx)
                                                        .map(|meta| meta.dehydrate())
                                                        .unwrap_or_default();
                                                    format!("{head}</head><body>{app}")
                                                }
                                            }));
                                            while let Some(fragment) = shell.next().await {
                                                tx.send(fragment).await;
                                            }
                                            tx.close_channel();
                                        })
                                        .await;
                                }
                            });
                    }
                });

                let stream = futures::stream::once(async move { head.clone() })
                    .chain(rx)
                    .chain(futures::stream::once(async { tail.to_string() }))
                    .map(|html| Ok(Bytes::from(html)) as io::Result<Bytes>);
                StreamBody::new(Box::pin(stream) as PinnedHtmlStream)
            }
        })
    }
}
