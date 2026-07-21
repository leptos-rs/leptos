#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, http::Request};
    use http_body_util::BodyExt;
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::{LeptosRoutes, generate_route_list_with_ssg};
    use leptos_meta::{MetaTags, provide_meta_context};
    use leptos_router::{
        SsrMode,
        components::{Route, Router as LeptosRouter, Routes},
        path,
        static_routes::StaticRoute,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

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
                                    use_context::<leptos_axum::ResponseOptions>()
                                {
                                    res.insert_header(
                                        axum::http::header::HeaderName::from_static(
                                            "x-render-epoch",
                                        ),
                                        axum::http::header::HeaderValue::from_str(
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

    #[tokio::test]
    async fn concurrent_static_regeneration_pairs_body_with_headers() {
        let site_root = std::env::temp_dir()
            .join(format!("leptos_axum_static_race_{}", std::process::id()));
        std::fs::create_dir_all(&site_root).unwrap();

        let options = LeptosOptions::builder()
            .output_name("static-race-repro")
            .site_root(site_root.to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();

        // Deliberately do NOT run the StaticRouteGenerator: the `.html` must
        // be missing so the first requests race down the on-demand
        // regeneration branch concurrently.
        let (routes, _generator) = generate_route_list_with_ssg(EpochApp);

        let app: Router = Router::new()
            .leptos_routes(&options, routes, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .with_state(options);

        let responses = futures::future::join_all((0..16).map(|_| {
            app.clone().oneshot(
                Request::builder()
                    .uri("/epoch")
                    .body(Body::empty())
                    .unwrap(),
            )
        }))
        .await;

        let mut mismatches = Vec::new();
        for (i, resp) in responses.into_iter().enumerate() {
            let resp = resp.unwrap();
            let status = resp.status();
            let header_epoch = resp
                .headers()
                .get("x-render-epoch")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            let html = String::from_utf8_lossy(&body).into_owned();
            let body_epoch = html
                .split("epoch-")
                .nth(1)
                .and_then(|tail| tail.split("-marker").next())
                .map(str::to_string);
            if header_epoch != body_epoch {
                mismatches.push(format!(
                    "response {i}: status={status} \
                     header_epoch={header_epoch:?} body_epoch={body_epoch:?}"
                ));
            }
        }

        let _ = std::fs::remove_dir_all(&site_root);

        assert!(
            mismatches.is_empty(),
            "body and x-render-epoch header must come from one render; \
             mismatched responses:\n{}",
            mismatches.join("\n")
        );
    }
}
