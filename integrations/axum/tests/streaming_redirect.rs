//! An integration test for a `<ProtectedRoute/>` whose condition comes from a blocking resource.
//! This needs to hold the head long enough to redirect under streaming SSR.

#![cfg(all(feature = "default", not(feature = "wasm")))]

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, http::Request};
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use leptos_meta::{MetaTags, provide_meta_context};
    use leptos_router::{
        components::{ProtectedRoute, Route, Router as LeptosRouter, Routes},
        path,
    };
    use tower::ServiceExt;

    #[component]
    fn RedirectApp() -> impl IntoView {
        provide_meta_context();

        let allowed = Resource::new_blocking(
            || (),
            |()| async {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                false
            },
        );

        view! {
            <LeptosRouter>
                <main>
                    <Routes fallback=|| view! { <p>"Not found."</p> }>
                        <ProtectedRoute
                            path=path!("guarded")
                            view=|| view! { <h1>"secret"</h1> }
                            condition=move || allowed.get()
                            redirect_path=|| "/public"
                            fallback=|| view! { <p>"Loading"</p> }
                        />
                        <Route path=path!("public") view=|| view! { <h1>"public"</h1> } />
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
                    <RedirectApp/>
                </body>
            </html>
        }
    }

    fn app() -> Router {
        let options = LeptosOptions::builder()
            .output_name("streaming-redirect")
            .site_root(std::env::temp_dir().to_string_lossy().to_string())
            .site_pkg_dir("pkg")
            .build();
        let routes = generate_route_list(RedirectApp);
        Router::new()
            .leptos_routes(&options, routes, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .with_state(options)
    }

    fn navigation(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("Accept", "text/html,application/xhtml+xml")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn protected_route_redirects_under_streaming_ssr() {
        for i in 0..25 {
            let resp = app().oneshot(navigation("/guarded")).await.unwrap();
            let status = resp.status();
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            assert_eq!(
                (status.as_u16(), location.as_deref()),
                (302, Some("/public")),
                "attempt {i} did not redirect"
            );
        }
    }

    #[tokio::test]
    async fn unguarded_route_is_untouched() {
        let resp = app().oneshot(navigation("/public")).await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        assert!(resp.headers().get("location").is_none());
    }
}
