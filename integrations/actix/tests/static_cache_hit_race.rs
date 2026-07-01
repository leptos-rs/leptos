#[cfg(test)]
mod tests {
    use actix_web::{App, test, web::Data};
    use futures::stream;
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_actix::{LeptosRoutes, generate_route_list_with_ssg};
    use leptos_meta::{MetaTags, provide_meta_context};
    use leptos_router::{
        SsrMode,
        components::{Route, Router as LeptosRouter, Routes},
        path,
        static_routes::StaticRoute,
    };
    use std::{
        sync::{
            LazyLock, Mutex,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };
    use tokio::sync::mpsc;

    // A large body makes each regeneration's file write take a while, widening
    // the window in which a concurrent cache hit can observe a render in
    // progress (one render's file on disk while another render's headers are
    // being published) — exactly the interval the cache-hit path must keep the
    // body and headers consistent across.
    const FILLER_BYTES: usize = 512 * 1024;

    // Drives the route's regeneration from the test. The receiver is installed
    // before the app runs and taken by the `regenerate` stream; dropping the
    // sender ends the stream so the background loop can wind down cleanly.
    static REGEN_RX: LazyLock<Mutex<Option<mpsc::Receiver<()>>>> =
        LazyLock::new(|| Mutex::new(None));

    // An ISR route whose regeneration is fed by `REGEN_RX`. Every cycle
    // re-renders (bumping the epoch), caches the new headers, then rewrites the
    // file — the two-step store the cache-hit branch reads back unsynchronized.
    #[component]
    fn EpochApp() -> impl IntoView {
        provide_meta_context();
        view! {
            <LeptosRouter>
                <main>
                    <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                        <Route
                            path=path!("/epoch")
                            ssr=SsrMode::Static(
                                StaticRoute::new().regenerate(|_| {
                                    let rx = REGEN_RX
                                        .lock()
                                        .unwrap()
                                        .take()
                                        .expect("regenerate called once");
                                    stream::unfold(rx, |mut rx| async move {
                                        rx.recv().await.map(|()| ((), rx))
                                    })
                                }),
                            )
                            view=|| {
                                static EPOCH: AtomicU64 = AtomicU64::new(0);
                                let epoch = EPOCH.fetch_add(1, Ordering::Relaxed);
                                if let Some(res) =
                                    use_context::<leptos_actix::ResponseOptions>()
                                {
                                    res.insert_header(
                                        actix_web::http::header::HeaderName::from_static(
                                            "x-render-epoch",
                                        ),
                                        actix_web::http::header::HeaderValue::from_str(
                                            &epoch.to_string(),
                                        )
                                        .unwrap(),
                                    );
                                }
                                // Filler has no `epoch-`/`-marker` substrings, so it
                                // can't confuse the body-epoch parser below.
                                let filler = "x".repeat(FILLER_BYTES);
                                let marker = format!("epoch-{epoch}-marker");
                                view! { <h1>{marker}</h1><p>{filler}</p> }
                            }
                        />
                    </Routes>
                </main>
            </LeptosRouter>
        }
    }

    fn shell(_options: LeptosOptions) -> impl IntoView {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <MetaTags/>
                </head>
                <body>
                    <EpochApp/>
                </body>
            </html>
        }
    }

    fn body_epoch(html: &str) -> Option<String> {
        html.split("epoch-")
            .nth(1)
            .and_then(|tail| tail.split("-marker").next())
            .map(str::to_string)
    }

    // The cache-*hit* branch reads the body (the file handle opened up front) and
    // the headers (a separate `STATIC_HEADERS` lookup) as two unsynchronized
    // operations, with no lock shared with `write_static_route`. While an
    // already-cached path is being regenerated, a request can therefore open one
    // render's file but read another render's headers.
    //
    // This test serves an ISR route, regenerates it continuously, and hammers it
    // with concurrent cache hits, asserting every response pairs its body with
    // its `x-render-epoch` header. It fails on the unsynchronized cache-hit
    // path: a request serves an old body with a newer render's epoch header.
    #[actix_web::test]
    async fn cache_hit_during_regeneration_pairs_body_with_headers() {
        // the render path spawns futures on the global executor
        let _ = any_spawner::Executor::init_tokio();

        let site_root = std::env::temp_dir().join(format!(
            "leptos_actix_cache_hit_race_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&site_root).unwrap();

        let options = LeptosOptions::builder()
            .output_name("static-cache-hit-race")
            .site_root(site_root.to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();

        // Capacity-1 channel: the pump is paced by the regeneration loop, so a
        // backlog can't build up and shutdown stays prompt. Install the receiver
        // before the app runs so `regenerate` can take it.
        let (tx, rx) = mpsc::channel::<()>(1);
        *REGEN_RX.lock().unwrap() = Some(rx);

        // Do not pre-run the generator: the first request generates the file and
        // spawns the background regeneration loop.
        let (routes, _generator) = generate_route_list_with_ssg(EpochApp);

        let app = test::init_service(
            App::new()
                .app_data(Data::new(options.clone()))
                .leptos_routes(routes, move || shell(options.clone())),
        )
        .await;

        // Kick off generation + the regeneration loop. Once this returns the
        // file exists, so every subsequent request takes the cache-hit branch.
        let _ = test::call_service(
            &app,
            test::TestRequest::get().uri("/epoch").to_request(),
        )
        .await;

        // Drive regeneration continuously, paced by the loop draining the
        // channel. Aborting the pump drops the only sender, which ends the
        // regeneration stream and lets the spawned task wind down.
        let pump =
            tokio::spawn(async move { while tx.send(()).await.is_ok() {} });

        // Hammer the cache-hit branch concurrently with the regeneration loop.
        // Stop as soon as a mismatch is seen (the bug is proven) or the budget
        // is exhausted (no mismatch -> the pairing held).
        const ROUNDS: usize = 80;
        const BATCH: usize = 16;
        let mut mismatches = Vec::new();
        'outer: for _ in 0..ROUNDS {
            let responses = futures::future::join_all((0..BATCH).map(|_| {
                test::call_service(
                    &app,
                    test::TestRequest::get().uri("/epoch").to_request(),
                )
            }))
            .await;
            for resp in responses {
                let header_epoch = resp
                    .headers()
                    .get("x-render-epoch")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                let body = test::read_body(resp).await;
                let html = String::from_utf8_lossy(&body).into_owned();
                let body_epoch = body_epoch(&html);
                // Every cache hit must pair its body with the matching
                // x-render-epoch header. A wrong header (the original bug) fails
                // here; so does an absent one (`None`) — publishing the headers
                // before the file is made visible means a hit should always find
                // them, so a `None` would signal that guarantee regressing.
                if header_epoch.as_deref() != body_epoch.as_deref() {
                    mismatches.push(format!(
                        "header_epoch={header_epoch:?} \
                         body_epoch={body_epoch:?}"
                    ));
                    break 'outer;
                }
            }
        }

        // Stop regeneration and let the spawned task finish before the runtime
        // is dropped, so no reactive work is left mid-flight at shutdown.
        pump.abort();
        let _ = pump.await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let _ = std::fs::remove_dir_all(&site_root);

        assert!(
            mismatches.is_empty(),
            "a cache hit served a body and x-render-epoch header from \
             different renders:\n{}",
            mismatches.join("\n")
        );
    }
}
