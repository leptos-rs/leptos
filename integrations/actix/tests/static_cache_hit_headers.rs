#[cfg(test)]
mod tests {
    use actix_web::{App, test, web::Data};
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_actix::{LeptosRoutes, generate_route_list_with_ssg};
    use leptos_meta::{MetaTags, provide_meta_context};
    use leptos_router::{
        SsrMode,
        components::{Route, Router as LeptosRouter, Routes},
        path,
        static_routes::StaticRoute,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    // Same epoch-stamping view as `static_race.rs`: every render bumps a global
    // epoch and stamps it into both an `x-render-epoch` header and the body.
    #[component]
    fn EpochApp() -> impl IntoView {
        provide_meta_context();
        view! {
            <LeptosRouter>
                <main>
                    <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                        <Route
                            path=path!("/epoch")
                            ssr=SsrMode::Static(StaticRoute::new())
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
                                let marker = format!("epoch-{epoch}-marker");
                                view! { <h1>{marker}</h1> }
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

    // The previous commit closed the cache-*miss* window: the on-demand branch
    // now serves the in-memory HTML paired with the headers from that same
    // render. But the cache-*hit* branch still reads the two halves of the
    // response independently — the body from the file handle opened up front,
    // the headers from a separate `STATIC_HEADERS` lookup.
    //
    // Worse, that lookup is not a snapshot: `STATIC_HEADERS` stores a
    // `ResponseOptions`, which is an `Arc<RwLock<ResponseParts>>`, and the
    // cache-hit branch `.cloned()`s it (cloning the `Arc`, not the contents).
    // `extend_response` then applies the headers with `std::mem::take`, draining
    // them out of the shared, still-cached `ResponseParts`. So the first caller
    // to serve the cached entry empties it, and every cache hit afterwards
    // serves the body with *no* custom headers.
    //
    // This test serves one route, then hits the cache twice, and asserts that
    // every response — generated or cached — pairs its body with its
    // `x-render-epoch` header.
    #[actix_web::test]
    async fn cache_hit_keeps_headers_paired_with_body() {
        // the render path spawns futures on the global executor
        let _ = any_spawner::Executor::init_tokio();

        let site_root = std::env::temp_dir().join(format!(
            "leptos_actix_cache_hit_headers_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&site_root).unwrap();

        let options = LeptosOptions::builder()
            .output_name("static-cache-hit-headers")
            .site_root(site_root.to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();

        // Do not pre-run the generator: the first request generates the file.
        let (routes, _generator) = generate_route_list_with_ssg(EpochApp);

        let app = test::init_service(
            App::new()
                .app_data(Data::new(options.clone()))
                .leptos_routes(routes, move || shell(options.clone())),
        )
        .await;

        // Three sequential requests: the first generates the file and caches its
        // headers; the next two are plain cache hits.
        let mut results = Vec::new();
        for _ in 0..3 {
            let resp = test::call_service(
                &app,
                test::TestRequest::get().uri("/epoch").to_request(),
            )
            .await;
            let header_epoch = resp
                .headers()
                .get("x-render-epoch")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let body = test::read_body(resp).await;
            let html = String::from_utf8_lossy(&body).into_owned();
            results.push((header_epoch, body_epoch(&html)));
        }

        let _ = std::fs::remove_dir_all(&site_root);

        let mut failures = Vec::new();
        for (i, (header_epoch, body_epoch)) in results.iter().enumerate() {
            // The body always carries the epoch it was rendered at; the cached
            // header for that same body must be present and equal.
            if header_epoch.as_deref() != body_epoch.as_deref() {
                failures.push(format!(
                    "request {i}: header_epoch={header_epoch:?} \
                     body_epoch={body_epoch:?}"
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "every response (generated or cached) must serve its body with \
             the matching x-render-epoch header:\n{}",
            failures.join("\n")
        );
    }
}
