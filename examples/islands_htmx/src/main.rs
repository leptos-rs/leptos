#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::get, Router};
    use islands_htmx::app::{page, Page};
    use leptos::prelude::*;
    use leptos_axum::{file_and_error_handler, render_app_to_stream};

    let conf = get_configuration(None).unwrap();
    let options = conf.leptos_options;
    let addr = options.site_addr;

    // Rendering spawns futures; `leptos_routes` would set the executor up, plain routes do it here.
    any_spawner::Executor::init_tokio().unwrap();

    // The pages are plain server routes; nothing routes in the browser.
    let route = |which: Page| {
        let options = options.clone();
        render_app_to_stream(move || page(options.clone(), which))
    };
    let app = Router::new()
        .route("/", get(route(Page::Home)))
        .route("/about", get(route(Page::About)))
        .route("/fragment", get(fragment))
        .fallback(file_and_error_handler(|options| {
            page(options, Page::NotFound)
        }))
        .with_state(options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

/// A fragment is HTML alone, rendered to a string: no shell, and none of the scripts a whole page carries.
#[cfg(feature = "ssr")]
async fn fragment() -> axum::response::Html<String> {
    use leptos::prelude::*;

    let owner = Owner::new_root(None);
    axum::response::Html(owner.with(|| islands_htmx::app::fragment().to_html()))
}

// The browser half is the library alone; cargo-leptos never runs this.
#[cfg(not(feature = "ssr"))]
fn main() {}
