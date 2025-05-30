# Tauri from scratch

This is a guide on how to build a leptos tauri project from scratch without using a template.

First

```sh
cargo new leptos_tauri_from_scratch
```

Then, make our two separate project folders. We need one for our actual app, _src-orig/_ and the other is required when using `cargo tauri`

```sh
mkdir src-orig && mkdir src-tauri
```

Delete the original src folder.

```sh
rm -r src
```

Rewrite the `Cargo.toml` file in our crate root to the following.

```toml
[workspace]
resolver = "2"
members = ["src-tauri", "src-orig"]

[profile.release]
codegen-units = 1
lto = true
```

We'll list our workspace members. `codegen-units = 1` and `lto = true` are good things to have for our eventual release, they make the wasm file smaller.

What we're going to do is use `cargo leptos` for building our SSR server and we'll call trunk from `cargo tauri` for building our CSR client that we bundle into our different apps.

Let's add a `Trunk.toml` file.

```toml
[build]
target = "./src-orig/index.html"

[watch]
ignore = ["./src-tauri"]
```

The target of `index.html` is what trunk uses to build the wasm and js files that we'll need for the bundling process when we call `cargo tauri build`. We'll get the resulting files in a `src-orig/dist` folder.

Create the `index.html` file

```sh
touch src-orig/index.html
```

Let's fill it with

```html
<!DOCTYPE html>
<html>
  <head>
    <link
      data-trunk
      rel="rust"
      data-wasm-opt="z"
      data-bin="leptos_tauri_from_scratch_bin"
    />
    <link rel="icon" type="image/x-icon" href="favicon.ico" />
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  </head>
  <body></body>
</html>
```

This line

```html
<link
  data-trunk
  rel="rust"
  data-wasm-opt="z"
  data-bin="leptos_tauri_from_scratch_bin"
/>
```

Tells trunk we want to compile our wasm to be small with `opt="z"` and that our binary will be named `"leptos_tauri_from_scratch_bin"`.

We need to specify that our binary will be a different name than our project name because we are also going to get a wasm file from our library and if we don't use different names then `cargo tauri` will get confused.

More specifically two wasm artifacts will be generated, one for the lib and the other for the binary and it won't know which to use.

Create a favicon that we referenced.

```sh
mkdir public
curl https://raw.githubusercontent.com/leptos-rs/leptos/main/examples/counter/public/favicon.ico > public/favicon.ico
```

Let's create a tauri configuration file.

```sh
touch src-tauri/taur.conf.json
```

And drop this in there

```json
{
  "identifier": "leptos.chat.app",
  "productName": "leptos_tauri_from_scratch",
  "version": "0.1.0",
  "build": {
    "beforeDevCommand": "",
    "beforeBuildCommand": "trunk build --no-default-features -v --features \"csr\"",
    "devUrl": "http://127.0.0.1:3000",
    "frontendDist": "../dist"
  },
  "bundle": {
    "active": true,
    "category": "DeveloperTool",
    "copyright": "",
    "externalBin": [],
    "icon": ["icons/icon.png"],
    "longDescription": "",
    "macOS": {
      "entitlements": null,
      "exceptionDomain": "",
      "frameworks": [],
      "providerShortName": null,
      "signingIdentity": null
    },
    "resources": [],
    "shortDescription": "",
    "targets": "all",
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  },
  "app": {
    "security": {
      "csp": null
    },
    "windows": [
      {
        "fullscreen": false,
        "height": 800,
        "resizable": true,
        "title": "LeptosChatApp",
        "width": 1200
      }
    ]
  }
}
```

You can basically ignore all of this except for

```json
  "build": {
    "beforeDevCommand": "",
    "beforeBuildCommand": "trunk build --no-default-features -v --features \"csr\"",
    "devUrl": "http://127.0.0.1:3000",
    "frontendDist": "../dist"
  },
```

Let's look at

```json
    "beforeBuildCommand": "trunk build --no-default-features -v --features \"csr\"",
```

When we `cargo tauri build` this will run before hand. Trunk will run it's build process, using the index.html file in the src-orig that we specified in `Trunk.toml`.

We'll build a binary using only the CSR feature. This is important.

We are going to build an SSR app, and serve it over the internet but we are also going to build a tauri client for desktop and mobile using CSR.

It's going to make network requests to our server that is servering our app to browsers using SSR.

This is the best of both worlds, we get the SEO of SSR and other advantages while being able to use CSR to build our app for other platforms.

```json
    "devUrl": "http://127.0.0.1:3000",
    "frontendDist": "../dist"
```

Check <https://tauri.app/v1/api/config/#buildconfig> for what these do, but our before build command `trunk build` will build into a folder `src-orig/dist` which we reference here.

Let's add a `Cargo.toml`` to both of our packages.

```sh
touch src-tauri/Cargo.toml && touch src-orig/Cargo.toml
```

Let's change `src-tauri/Cargo.toml` to this.

```toml
[package]
name = "src_tauri"
version = "0.0.1"
edition = "2021"

[lib]
name = "app_lib"
path = "src/lib.rs"

[build-dependencies]
tauri-build = { version = "2.2.0", features = [] }

[dependencies]
log = "0.4.22"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2.5.1", features = ["devtools"] }
tauri-plugin-http = "2.4.4"

[features]
#default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

To make use of `cargo tauri build` we need `tauri-build` and we also need a `build.rs`.

```sh
touch src-tauri/build.rs
```

And let's change that to

```rust
fn main() {
    tauri_build::build();
}
```

In our `src-orig/Cargo.toml` let's add.

```toml
[package]
name = "leptos_tauri_from_scratch"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib", "rlib"]

[[bin]]
name = "leptos_tauri_from_scratch_bin"
path = "./src/main.rs"

[dependencies]
axum = { version = "0.8.4", optional = true }
axum-macros = { version = "0.5.0", optional = true }
console_error_panic_hook = { version = "0.1.7", optional = true }
leptos = { git = "https://github.com/leptos-rs/leptos.git", rev = "v0.8.2" }
leptos_axum = { git = "https://github.com/leptos-rs/leptos.git", rev = "v0.8.2", optional = true }
leptos_meta = { git = "https://github.com/leptos-rs/leptos.git", rev = "v0.8.2", optional = true }
server_fn = { git = "https://github.com/leptos-rs/leptos.git", rev = "v0.8.2", optional = true }
tokio = { version = "1.45.1", features = ["rt-multi-thread"], optional = true }
tower = { version = "0.5.2", optional = true }
tower-http = { version = "0.5.2", features = ["fs", "cors"], optional = true }
wasm-bindgen = { version = "=0.2.100", optional = true }

[features]
csr = ["leptos/csr", "dep:server_fn"]
hydrate = [
  "leptos/hydrate",
  "dep:leptos_meta",
  "dep:console_error_panic_hook",
  "dep:wasm-bindgen"
]
ssr = [
  "dep:axum",
  "dep:axum-macros",
  "leptos/ssr",
  "dep:leptos_axum",
  "dep:leptos_meta",
  "leptos_meta/ssr",
  "dep:tower-http",
  "dep:tower",
  "dep:tokio",
]

[package.metadata.leptos]
bin-exe-name = "leptos_tauri_from_scratch_bin"
output-name = "leptos_tauri_from_scratch"
assets-dir = "../public"
site-pkg-dir = "pkg"
site-root = "target/site"
site-addr = "0.0.0.0:3000"
reload-port = 3001
browserquery = "defaults"
watch = false
env = "DEV"
bin-features = ["ssr"]
bin-default-features = false
lib-features = ["hydrate"]
lib-default-features = false
```

So this looks like a normal SSR leptos, except for our CSR, Hydrate, and SSR versions.

```toml
csr = ["leptos/csr", "dep:server_fn"]
hydrate = [
  "leptos/hydrate",
  "dep:leptos_meta",
  "dep:console_error_panic_hook",
  "dep:wasm-bindgen"
]
ssr = [
```

also our binary is specified and named

```toml
[[bin]]
name="leptos_tauri_from_scratch_bin"
path="./src/main.rs"
```

our lib is specified, but unnamed (it will default to the project name in cargo leptos and in cargo tauri). We need the different crate types for `cargo leptos serve` and `cargo tauri build`

```toml
[lib]
crate-type = ["staticlib", "cdylib", "rlib"]
```

We've added the override to our cargo leptos metadata.

```toml
[package.metadata.leptos]
bin-exe-name="leptos_tauri_from_scratch_bin"
```

Our tauri app is going to send server function calls to this address, this is where we'll serve our hydratable SSR client from.

```toml
site-addr = "0.0.0.0:3000"
```

Now let's create the `main.rs` that we reference in the `src-orig/Cargo.toml`

```sh
mkdir src-orig/src && touch src-orig/src/main.rs
```

and drop this in there...

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        body::Body,
        extract::{Request, State},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_tauri_from_scratch::{
        app::{shell, App},
        fallback::file_and_error_handler,
    };
    use tower_http::cors::CorsLayer;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    #[derive(Clone, Debug, axum_macros::FromRef)]
    pub struct ServerState {
        pub options: LeptosOptions,
        pub routes: Vec<leptos_axum::AxumRouteListing>,
    }

    let state = ServerState {
        options: leptos_options,
        routes: routes.clone(),
    };

    pub async fn server_fn_handler(
        State(state): State<ServerState>,
        request: Request<Body>,
    ) -> impl IntoResponse {
        leptos_axum::handle_server_fns_with_context(
            move || {
                provide_context(state.clone());
            },
            request,
        )
        .await
        .into_response()
    }

    let cors = CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_origin(
            // Allow requests from the Tauri app
            "tauri://localhost"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_headers(vec![
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    pub async fn leptos_routes_handler(
        State(state): State<ServerState>,
        req: Request<Body>,
    ) -> axum::response::Response {
        let leptos_options = state.options.clone();
        let handler = leptos_axum::render_route_with_context(
            state.routes.clone(),
            move || {
                provide_context("...");
            },
            move || shell(leptos_options.clone()),
        );
        handler(axum::extract::State(state), req)
            .await
            .into_response()
    }

    let app = Router::new()
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .layer(cors)
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler)
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "csr")]
pub fn main() {
    server_fn::client::set_server_url("http://127.0.0.1:3000");
    leptos::mount::mount_to_body(leptos_tauri_from_scratch::app::App);
}
```

and the hydration at `src-orig/src/lib.rs`

```rust
pub mod app;
#[cfg(feature = "ssr")]
pub mod fallback;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
```

This is our three pronged binary.

When we run cargo leptos server, we're going to get a server that is what's under `#[cfg(feature="ssr")]`.

And our csr feature

```rust
#[cfg(feature = "csr")]
pub fn main() {
    server_fn::client::set_server_url("http://127.0.0.1:3000");
    leptos::mount::mount_to_body(leptos_tauri_from_scratch::app::App);
}
```

Here we're setting the server functions to use the url base that we access in our browser. I.e local host, on the port we specified in the leptos metadata.
Otherwise our tauri app will try to route server function network requests using it's own idea of what it's url is. Which is `tauri://localhost` on macOS, and something else on windows.

Since we are going to be getting API requests from different locations beside our server's domain let's set up CORS, if you don't do this your tauri apps won't be able to make server function calls because it will run into CORS erros.

```rust
        let cors = CorsLayer::new()
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_origin(
                "tauri://localhost"
                    .parse::<axum::http::HeaderValue>()
                    .unwrap(),
            )
            .allow_headers(vec![axum::http::header::CONTENT_TYPE]);
```

If you are on windows the origin of your app will be different than `tauri://localhost` and you'll need to figure that out, as well as if you deploy it to places that aren't your localhost!

Everything else is standard leptos, so let's fill in the fallback and the lib really quick.

```sh
touch src-orig/src/lib.rs && touch src-orig/src/fallback.rs
```

Let's dump this bog standard leptos code in the `src-orig/src/app.rs`

```rust
use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos_meta::MetaTags;
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server(endpoint = "hello_world")]
pub async fn hello_world_server() -> Result<String, ServerFnError> {
    Ok("Hey.".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let action = ServerAction::<HelloWorldServer>::new();
    let vals = RwSignal::new(String::new());
    Effect::new(move |_| {
        if let Some(resp) = action.value().get() {
            match resp {
                Ok(val) => vals.set(val),
                Err(err) => vals.set(format!("{err:?}")),
            }
        }
    });

    view! {
        <button
            on:click=move |_| {
                action.dispatch(HelloWorldServer{});
            }
        >"Hello world."</button>
        <br/><br/>
        <span>"Server says: "</span>
        {move || vals.get()}
    }
}
```

and add this to `src-org/src/fallback.rs`

```rust
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, Uri},
    response::{IntoResponse, Response as AxumResponse},
};
use leptos::{view, prelude::LeptosOptions};
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn file_and_error_handler(
    uri: Uri,
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> AxumResponse {
    let root = options.site_root.clone();
    let res = get_static_file(uri.clone(), &root).await.unwrap();

    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        let handler = leptos_axum::render_app_to_stream(
            move || view! {404},
        );
        handler(req).await.into_response()
    }
}

async fn get_static_file(
    uri: Uri,
    root: &str,
) -> Result<Response<Body>, (StatusCode, String)> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .unwrap();
    match ServeDir::new(root).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {err}"),
        )),
    }
}
```

Let's fill in our `src-tauri/src/` folder.

```sh
mkdir src-tauri/src && touch src-tauri/src/main.rs && touch src-tauri/src/lib.rs
```

and drop this in `src-tauri/src/main.rs` This is standard tauri boilerplate.

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    app_lib::run();
}
```

and in `src-tauri/src/lib.rs`

```rust
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

We're gonna open devtools right away to see what is going on in our app. We need the tauri_http_plugin to make http calls, and generate_context reads our `tauri.conf.json` in the package in which its run.

We need an icon folder and an icon to build.

```sh
mkdir src-tauri/icons
curl https://raw.githubusercontent.com/tauri-apps/tauri/dev/examples/.icons/128x128.png > src-tauri/icons/icon.png
```

set nightly

```sh
rustup override set nightly
```

Then run

```sh
cargo leptos serve
```

You should get something like

```sh
➜  lepto_tauri_from_scratch git:(main) ✗ cargo leptos serve
    Finished dev [unoptimized + debuginfo] target(s) in 0.60s
       Cargo finished cargo build --package=leptos_tauri_from_scratch --lib --target-dir=/Users/sam/Projects/lepto_tauri_from_scratch/target/front --target=wasm32-unknown-unknown --no-default-features --features=hydrate
       Front compiling WASM
    Finished dev [unoptimized + debuginfo] target(s) in 0.93s
       Cargo finished cargo build --package=leptos_tauri_from_scratch --bin=leptos_tauri_from_scratch_bin --no-default-features --features=ssr
     Serving at http://0.0.0.0:3000
listening on http://0.0.0.0:3000
```

Now open a new terminal and

```sh
cargo tauri build
```

> Install `tauri-cli` if you haven't already.

It'll build with csr before

```sh
Running beforeBuildCommand `trunk build --no-default-features -v --features "csr"`
```

and then you should have your app, I'm on macOS so here's what I get. It's for desktop.

```sh
 Compiling src_tauri v0.0.1 (/Users/sam/Projects/lepto_tauri_from_scratch/src-tauri)
    Finished release [optimized] target(s) in 2m 26s
    Bundling leptos_tauri_from_scratch.app (/Users/sam/Projects/lepto_tauri_from_scratch/target/release/bundle/macos/leptos_tauri_from_scratch.app)
    Bundling leptos_tauri_from_scratch_0.1.0_x64.dmg (/Users/sam/Projects/lepto_tauri_from_scratch/target/release/bundle/dmg/leptos_tauri_from_scratch_0.1.0_x64.dmg)
    Running bundle_dmg.sh
```

Open run it and voilá. Click hello world button and read "Hey" from the server.

## Thoughts, Feedback, Criticism, Comments?

Send me any of the above, I'm @sjud on leptos discord. I'm always looking to improve and make these projects more helpful for the community. So please let me know how I can do that. Thanks!
