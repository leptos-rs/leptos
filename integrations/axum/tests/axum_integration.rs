use reqwest::{Client, StatusCode, Url};
use std::{
    path::Path,
    sync::Once,
    time::{Duration, Instant},
};
use tokio::process::{Child, Command};

#[tokio::test]
async fn bare_no_fallback() -> anyhow::Result<()> {
    let host = "127.0.0.1:3010";
    let _service = start_test_service("service_mode", host, "bare").await;
    let url = url(host);
    let client = Client::new();
    // this version has no fallbacks attached, so no other response, no error page.
    let res = client.get(url.join("/pkg/service_mode.js")?).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(res.content_length(), Some(0));
    Ok(())
}

#[tokio::test]
async fn fallback() -> anyhow::Result<()> {
    let host = "127.0.0.1:3020";
    let _service = start_test_service("service_mode", host, "fallback").await;
    let url = url(host);
    let client = Client::new();
    // should provide the two site artifacts.
    let res = client.get(url.join("/pkg/service_mode.js")?).send().await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    let res = client
        .get(url.join("/pkg/service_mode.wasm")?)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    // the basic fallback will also have a shell to render the 404 Not Found
    let res = client.get(url.join("/pkg/no_such_path")?).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_ne!(res.content_length(), Some(0));
    assert!(res
        .text()
        .await?
        .contains("<title>Error from fallback</title>"));
    Ok(())
}

#[tokio::test]
async fn error_handler_service() -> anyhow::Result<()> {
    let host = "127.0.0.1:3040";
    let _service =
        start_test_service("service_mode", host, "error-handler-service").await;
    let url = url(host);
    let client = Client::new();
    // no site artifact, but has the error page as only the error handler is applied
    let res = client.get(url.join("/pkg/service_mode.js")?).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_ne!(res.content_length(), Some(0));
    assert!(res
        .text()
        .await?
        .contains("<title>Error from fallback</title>"));
    Ok(())
}

#[tokio::test]
async fn error_handler_service_fallback() -> anyhow::Result<()> {
    let host = "127.0.0.1:3050";
    let _service = start_test_service(
        "service_mode",
        host,
        "error-handler-service-fallback",
    )
    .await;
    let url = url(host);
    let client = Client::new();
    // should provide the two site artifacts.
    let res = client.get(url.join("/pkg/service_mode.js")?).send().await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    let res = client
        .get(url.join("/pkg/service_mode.wasm")?)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    // this composed service falback setup is similar to the basic non-service fallback setup.
    let res = client.get(url.join("/pkg/no_such_path")?).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_ne!(res.content_length(), Some(0));
    assert!(res
        .text()
        .await?
        .contains("<title>Error from fallback</title>"));
    Ok(())
}

#[tokio::test]
async fn route_site_pkg_no_fallback() -> anyhow::Result<()> {
    let host = "127.0.0.1:3060";
    let _service =
        start_test_service("service_mode", host, "route-site-pkg-no-fallback")
            .await;
    let url = url(host);
    let client = Client::new();
    // should provide the two site artifacts.
    let res = client.get(url.join("/pkg/service_mode.js")?).send().await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    let res = client
        .get(url.join("/pkg/service_mode.wasm")?)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    assert_ne!(res.content_length(), Some(0));
    // there is no fallback assigned to the routes under /pkg/ under this setup, so no error page
    let res = client.get(url.join("/pkg/no_such_path")?).send().await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(res.content_length(), Some(0));
    // however, the fallback service will trigger for all other unrouted paths.
    let res = client
        .get(url.join("/no_such_path_elsewhere")?)
        .send()
        .await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_ne!(res.content_length(), Some(0));
    assert!(res
        .text()
        .await?
        .contains("<title>Error from fallback</title>"));
    Ok(())
}

static BUILDER: Once = Once::new();

// Killing `cargo leptos watch` may not necessarily kill the underlying server task, so rather
// than running that, build and run the service in separate steps.  This also has the advantage
// of avoiding parallel build issues with generating the site onto the same location.
fn build_test_service(name: &str) {
    // this assumes the current working dir is at the root of this crate, i.e. `integration/axum`.
    let working_dir = Path::new("tests").join(name);

    let cmd = Command::new("cargo");
    let mut build = cmd
        .into_std()
        .arg("leptos")
        .arg("build")
        // need to manually specify this to avoid mismatch between this value that may be set (e.g.
        // during CI) and the `output-name` defined in Cargo.toml for this relevant project.
        .env("LEPTOS_OUTPUT_NAME", name)
        .current_dir(&working_dir)
        .spawn()
        .expect("cargo leptos build should start");
    if !build
        .wait()
        .expect("there shouldn't be i/o error")
        .success()
    {
        panic!("failed to run `cargo leptos build`");
    }
}

async fn start_test_service(name: &str, host: &str, mode: &str) -> Child {
    BUILDER.call_once(|| build_test_service("service_mode"));

    // this assumes the current working dir is at the root of this crate, i.e. `integration/axum`.
    let working_dir = Path::new("tests").join(name);

    let child = Command::new(Path::new("target").join("debug").join(name))
        .arg(mode)
        .kill_on_drop(true)
        .current_dir(&working_dir)
        .env("LEPTOS_SITE_ADDR", host)
        // need to manually specify this to avoid mismatch between this value that may be set (e.g.
        // during CI) and the `output-name` defined in Cargo.toml for this relevant project.
        .env("LEPTOS_OUTPUT_NAME", name)
        .spawn()
        .expect("the service should have been built and can start");

    let start_time = Instant::now();

    let url = format!("http://{host}");
    while start_time.elapsed() < Duration::from_secs(300) {
        if reqwest::get(&url).await.is_ok() {
            return child;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    panic!("The web server did not become ready within the expected time.");
}

fn url(host: &str) -> Url {
    format!("http://{host}").parse().expect("normal valid host")
}
