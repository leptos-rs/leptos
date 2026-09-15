#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
    };
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
    use tower::ServiceExt;

    #[component]
    fn App() -> impl IntoView {
        provide_meta_context();
        view! {
            <LeptosRouter>
                <main>
                    <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                        <Route
                            path=path!("/static")
                            ssr=SsrMode::Static(StaticRoute::new())
                            view=|| view! { <h1>"static body"</h1> }
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
                    <App/>
                </body>
            </html>
        }
    }

    async fn get(
        app: &Router,
        conditional: Option<(header::HeaderName, &str)>,
    ) -> axum::response::Response {
        let mut req = Request::builder().uri("/static");
        if let Some((name, value)) = conditional {
            req = req.header(name, value);
        }
        app.clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn body_len(response: axum::response::Response) -> usize {
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .len()
    }

    // The cache-hit path serves the file directly (not via `tower_http::ServeFile`)
    // so it can tie the body to the render that produced it. This test checks it
    // still does conditional-request handling: a cached hit advertises `ETag` and
    // `Last-Modified`, and a matching `If-None-Match`/`If-Modified-Since`
    // revalidates to `304 Not Modified` without resending the body.
    #[tokio::test]
    async fn cache_hit_supports_conditional_requests() {
        let site_root = std::env::temp_dir().join(format!(
            "leptos_axum_static_conditional_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&site_root).unwrap();

        let options = LeptosOptions::builder()
            .output_name("static-conditional")
            .site_root(site_root.to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();

        let (routes, _generator) = generate_route_list_with_ssg(App);
        let app: Router = Router::new()
            .leptos_routes(&options, routes, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .with_state(options);

        // First request generates the file (on-demand branch); the second is a
        // cache hit that carries the validators.
        let _ = get(&app, None).await;
        let cached = get(&app, None).await;
        assert_eq!(cached.status(), StatusCode::OK);
        let etag = cached
            .headers()
            .get(header::ETAG)
            .expect("cache hit must set an ETag")
            .to_str()
            .unwrap()
            .to_owned();
        let last_modified = cached
            .headers()
            .get(header::LAST_MODIFIED)
            .expect("cache hit must set Last-Modified")
            .to_str()
            .unwrap()
            .to_owned();
        assert!(body_len(cached).await > 0, "200 must carry the body");

        // Matching If-None-Match -> 304 with no body.
        let not_modified =
            get(&app, Some((header::IF_NONE_MATCH, &etag))).await;
        assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(body_len(not_modified).await, 0, "304 must have no body");

        // `*` matches any current representation.
        let star = get(&app, Some((header::IF_NONE_MATCH, "*"))).await;
        assert_eq!(star.status(), StatusCode::NOT_MODIFIED);

        // Matching If-Modified-Since -> 304.
        let not_modified_since =
            get(&app, Some((header::IF_MODIFIED_SINCE, &last_modified))).await;
        assert_eq!(not_modified_since.status(), StatusCode::NOT_MODIFIED);

        // A non-matching ETag still serves the full body.
        let stale =
            get(&app, Some((header::IF_NONE_MATCH, "\"nonsense\""))).await;
        assert_eq!(stale.status(), StatusCode::OK);
        assert!(body_len(stale).await > 0);

        let _ = std::fs::remove_dir_all(&site_root);
    }
}
