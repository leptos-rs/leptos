//! Leptos Enhanced Project Initialization
//! 
//! Provides intelligent project setup with automatic configuration,
//! smart templates, and compile-time validation.

pub mod cli;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Project template types with intelligent defaults
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
pub enum ProjectTemplate {
    /// Client-side only (SPA)
    Spa,
    /// Server + Client with hydration (most common)
    Fullstack,
    /// Static site generation
    Static,
    /// Server functions only (API)
    Api,
    /// Interactive wizard for custom setup
    Custom,
}

/// Server backend options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
pub enum ServerBackend {
    Axum,
    Actix,
    Warp,
}

/// Database integration options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Database {
    None,
    Sqlite,
    Postgresql,
    Mysql,
}

/// Styling framework options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Styling {
    None,
    Tailwind,
    VanillaCss,
    Sass,
}

/// Comprehensive project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitConfig {
    pub name: String,
    pub template: ProjectTemplate,
    pub server: ServerBackend,
    pub database: Database,
    pub styling: Styling,
    pub features: Vec<String>,
    pub use_tracing: bool,
    pub use_islands: bool,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            name: "my-leptos-app".to_string(),
            template: ProjectTemplate::Fullstack,
            server: ServerBackend::Axum,
            database: Database::None,
            styling: Styling::None,
            features: vec![],
            use_tracing: true,
            use_islands: false,
        }
    }
}

impl InitConfig {
    /// Create configuration for specific template with smart defaults
    pub fn for_template(name: String, template: ProjectTemplate) -> Self {
        let mut config = Self {
            name,
            template: template.clone(),
            ..Default::default()
        };

        // Set template-specific defaults
        match template {
            ProjectTemplate::Spa => {
                config.features = vec!["csr".to_string()];
                config.use_tracing = false;
            }
            ProjectTemplate::Fullstack => {
                config.features = vec!["ssr".to_string(), "hydrate".to_string()];
                config.database = Database::Sqlite; // Common default
            }
            ProjectTemplate::Static => {
                config.features = vec!["ssr".to_string()];
                config.use_tracing = false;
            }
            ProjectTemplate::Api => {
                config.features = vec!["ssr".to_string()];
                config.styling = Styling::None;
            }
            ProjectTemplate::Custom => {
                // Will be configured via interactive wizard
            }
        }

        config
    }

    /// Get features for client build
    pub fn client_features(&self) -> Vec<String> {
        match self.template {
            ProjectTemplate::Spa => vec!["csr".to_string()],
            ProjectTemplate::Fullstack => vec!["hydrate".to_string()],
            ProjectTemplate::Static => vec!["hydrate".to_string()],
            ProjectTemplate::Api => vec![], // No client build
            ProjectTemplate::Custom => self.features.clone(),
        }
    }

    /// Get features for server build  
    pub fn server_features(&self) -> Vec<String> {
        match self.template {
            ProjectTemplate::Spa => vec![], // No server build
            ProjectTemplate::Fullstack => vec!["ssr".to_string()],
            ProjectTemplate::Static => vec!["ssr".to_string()],
            ProjectTemplate::Api => vec!["ssr".to_string()],
            ProjectTemplate::Custom => self.features.clone(),
        }
    }

    /// Generate dependencies based on configuration
    pub fn dependencies(&self) -> HashMap<String, String> {
        let mut deps = HashMap::new();

        // Core Leptos
        let mut leptos_features = vec![];
        if self.use_tracing {
            leptos_features.push("tracing");
        }
        if self.use_islands {
            leptos_features.push("islands");
        }

        deps.insert(
            "leptos".to_string(),
            format!(
                r#"{{ version = "0.8", features = [{}] }}"#,
                leptos_features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        // Server dependencies
        if self.template != ProjectTemplate::Spa {
            match self.server {
                ServerBackend::Axum => {
                    deps.insert("leptos_axum".to_string(), r#"{ version = "0.8", optional = true }"#.to_string());
                    deps.insert("axum".to_string(), r#"{ version = "0.8", optional = true }"#.to_string());
                    deps.insert("tower".to_string(), r#"{ version = "0.5", optional = true }"#.to_string());
                    deps.insert("tower-http".to_string(), r#"{ version = "0.6", features = ["fs"], optional = true }"#.to_string());
                    deps.insert("tokio".to_string(), r#"{ version = "1.0", features = ["full"], optional = true }"#.to_string());
                }
                ServerBackend::Actix => {
                    deps.insert("leptos_actix".to_string(), r#"{ version = "0.8", optional = true }"#.to_string());
                    deps.insert("actix-web".to_string(), r#"{ version = "4.0", optional = true }"#.to_string());
                    deps.insert("actix-files".to_string(), r#"{ version = "0.6", optional = true }"#.to_string());
                }
                ServerBackend::Warp => {
                    deps.insert("leptos_warp".to_string(), r#"{ version = "0.8", optional = true }"#.to_string());
                    deps.insert("warp".to_string(), r#"{ version = "0.3", optional = true }"#.to_string());
                }
            }
        }

        // Database dependencies
        match self.database {
            Database::Sqlite => {
                deps.insert("sqlx".to_string(), r#"{ version = "0.8", features = ["runtime-tokio-rustls", "sqlite"], optional = true }"#.to_string());
            }
            Database::Postgresql => {
                deps.insert("sqlx".to_string(), r#"{ version = "0.8", features = ["runtime-tokio-rustls", "postgres"], optional = true }"#.to_string());
            }
            Database::Mysql => {
                deps.insert("sqlx".to_string(), r#"{ version = "0.8", features = ["runtime-tokio-rustls", "mysql"], optional = true }"#.to_string());
            }
            Database::None => {}
        }

        // Client-side dependencies
        if self.template != ProjectTemplate::Api {
            deps.insert("console_log".to_string(), "\"1.0\"".to_string());
            deps.insert("console_error_panic_hook".to_string(), "\"0.1\"".to_string());
            deps.insert("wasm-bindgen".to_string(), "\"0.2\"".to_string());
        }

        // Routing (most projects need it)
        if self.template != ProjectTemplate::Api {
            deps.insert("leptos_router".to_string(), r#"{ version = "0.8" }"#.to_string());
        }

        deps
    }

    /// Generate feature flags configuration
    pub fn feature_flags(&self) -> HashMap<String, Vec<String>> {
        let mut features = HashMap::new();

        match self.template {
            ProjectTemplate::Spa => {
                features.insert("default".to_string(), vec!["csr".to_string()]);
                features.insert("csr".to_string(), vec!["leptos/csr".to_string()]);
            }
            ProjectTemplate::Fullstack => {
                features.insert("default".to_string(), vec![]);
                features.insert("hydrate".to_string(), vec!["leptos/hydrate".to_string()]);
                
                let mut ssr_features = vec!["leptos/ssr".to_string()];
                match self.server {
                    ServerBackend::Axum => {
                        ssr_features.extend(vec![
                            "dep:leptos_axum".to_string(),
                            "dep:axum".to_string(),
                            "dep:tower".to_string(),
                            "dep:tower-http".to_string(),
                            "dep:tokio".to_string(),
                        ]);
                    }
                    ServerBackend::Actix => {
                        ssr_features.extend(vec![
                            "dep:leptos_actix".to_string(),
                            "dep:actix-web".to_string(),
                            "dep:actix-files".to_string(),
                        ]);
                    }
                    ServerBackend::Warp => {
                        ssr_features.extend(vec![
                            "dep:leptos_warp".to_string(),
                            "dep:warp".to_string(),
                        ]);
                    }
                }

                if self.database != Database::None {
                    ssr_features.push("dep:sqlx".to_string());
                }

                features.insert("ssr".to_string(), ssr_features);
            }
            ProjectTemplate::Static => {
                features.insert("default".to_string(), vec!["ssr".to_string()]);
                features.insert("ssr".to_string(), vec!["leptos/ssr".to_string()]);
            }
            ProjectTemplate::Api => {
                features.insert("default".to_string(), vec!["ssr".to_string()]);
                let mut ssr_features = vec!["leptos/ssr".to_string()];
                // Add server backend dependencies
                match self.server {
                    ServerBackend::Axum => {
                        ssr_features.extend(vec![
                            "dep:leptos_axum".to_string(),
                            "dep:axum".to_string(),
                            "dep:tokio".to_string(),
                        ]);
                    }
                    ServerBackend::Actix => {
                        ssr_features.extend(vec![
                            "dep:leptos_actix".to_string(), 
                            "dep:actix-web".to_string(),
                        ]);
                    }
                    ServerBackend::Warp => {
                        ssr_features.extend(vec![
                            "dep:leptos_warp".to_string(),
                            "dep:warp".to_string(),
                        ]);
                    }
                }
                features.insert("ssr".to_string(), ssr_features);
            }
            ProjectTemplate::Custom => {
                // Custom features will be configured via wizard
                features.insert("default".to_string(), vec![]);
            }
        }

        features
    }

    /// Generate leptos metadata configuration
    pub fn leptos_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();

        metadata.insert("output-name".to_string(), self.name.clone());
        metadata.insert("site-root".to_string(), "target/site".to_string());
        metadata.insert("site-pkg-dir".to_string(), "pkg".to_string());
        metadata.insert("site-addr".to_string(), "127.0.0.1:3000".to_string());
        metadata.insert("reload-port".to_string(), "3001".to_string());
        metadata.insert("browserquery".to_string(), "defaults".to_string());
        metadata.insert("watch".to_string(), "false".to_string());
        metadata.insert("env".to_string(), "DEV".to_string());

        // Template-specific metadata
        match self.template {
            ProjectTemplate::Spa => {
                metadata.insert("lib-features".to_string(), "[\"csr\"]".to_string());
                metadata.insert("lib-default-features".to_string(), "false".to_string());
            }
            ProjectTemplate::Fullstack => {
                metadata.insert("bin-features".to_string(), "[\"ssr\"]".to_string());
                metadata.insert("bin-default-features".to_string(), "false".to_string());
                metadata.insert("lib-features".to_string(), "[\"hydrate\"]".to_string());
                metadata.insert("lib-default-features".to_string(), "false".to_string());
            }
            ProjectTemplate::Static => {
                metadata.insert("bin-features".to_string(), "[\"ssr\"]".to_string());
                metadata.insert("bin-default-features".to_string(), "false".to_string());
            }
            ProjectTemplate::Api => {
                metadata.insert("bin-features".to_string(), "[\"ssr\"]".to_string());
                metadata.insert("bin-default-features".to_string(), "false".to_string());
            }
            ProjectTemplate::Custom => {
                // Will be configured via wizard
            }
        }

        // Add styling configuration
        if self.styling == Styling::Tailwind {
            metadata.insert("style-file".to_string(), "src/styles/tailwind.css".to_string());
        }

        metadata
    }
}

/// Project generator that creates complete, working projects
pub struct ProjectGenerator {
    config: InitConfig,
    target_path: PathBuf,
}

impl ProjectGenerator {
    pub fn new(config: InitConfig, target_path: impl AsRef<Path>) -> Self {
        Self {
            config,
            target_path: target_path.as_ref().to_path_buf(),
        }
    }

    /// Generate complete project structure
    pub fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.create_directories()?;
        self.generate_cargo_toml()?;
        self.generate_source_files()?;
        self.generate_assets()?;
        self.generate_configuration_files()?;
        self.setup_validation_system()?;

        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.target_path)?;
        std::fs::create_dir_all(self.target_path.join("src"))?;
        
        if self.config.template != ProjectTemplate::Api {
            std::fs::create_dir_all(self.target_path.join("public"))?;
        }

        if self.config.styling == Styling::Tailwind {
            std::fs::create_dir_all(self.target_path.join("src/styles"))?;
        }

        Ok(())
    }

    fn generate_cargo_toml(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cargo_toml = self.build_cargo_toml();
        std::fs::write(self.target_path.join("Cargo.toml"), cargo_toml)?;
        Ok(())
    }

    fn build_cargo_toml(&self) -> String {
        let mut toml = String::new();

        // Package section
        toml.push_str(&format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

"#,
            self.config.name
        ));

        // Lib section for non-API projects
        if self.config.template != ProjectTemplate::Api {
            toml.push_str(r#"[lib]
crate-type = ["cdylib", "rlib"]

"#);
        }

        // Dependencies
        toml.push_str("[dependencies]\n");
        for (name, version) in self.config.dependencies() {
            toml.push_str(&format!("{} = {}\n", name, version));
        }
        toml.push_str("\n");

        // Features
        toml.push_str("[features]\n");
        for (name, features) in self.config.feature_flags() {
            let features_str = features
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            toml.push_str(&format!("{} = [{}]\n", name, features_str));
        }
        toml.push_str("\n");

        // Leptos metadata
        toml.push_str("[package.metadata.leptos]\n");
        for (key, value) in self.config.leptos_metadata() {
            if value.starts_with('[') || value.starts_with("true") || value.starts_with("false") || value.chars().all(|c| c.is_ascii_digit()) {
                toml.push_str(&format!("{} = {}\n", key, value));
            } else {
                toml.push_str(&format!("{} = \"{}\"\n", key, value));
            }
        }

        toml
    }

    fn generate_source_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.config.template {
            ProjectTemplate::Spa => self.generate_spa_sources()?,
            ProjectTemplate::Fullstack => self.generate_fullstack_sources()?,
            ProjectTemplate::Static => self.generate_static_sources()?,
            ProjectTemplate::Api => self.generate_api_sources()?,
            ProjectTemplate::Custom => {
                // Custom template generation
            }
        }
        Ok(())
    }

    fn generate_spa_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Main.rs for SPA
        let main_rs = r#"use leptos::prelude::*;

mod app;
use app::App;

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("error initializing log");

    mount_to_body(App)
}
"#;
        std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;

        // App component
        let app_rs = r#"use leptos::prelude::*;
use leptos_router::{components::*, path};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="navbar">
                <A href="/">"Home"</A>
                <A href="/about">"About"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/about") view=AboutPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Count: " {count}
        </button>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <h1>"About"</h1>
        <p>"This is a Leptos SPA created with leptos init."</p>
    }
}
"#;
        std::fs::write(self.target_path.join("src/app.rs"), app_rs)?;

        Ok(())
    }

    fn generate_fullstack_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Main.rs for fullstack
        let main_rs = r#"#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing::get};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_http::services::ServeDir;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(app::App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .nest_service("/pkg", ServeDir::new("target/site/pkg"))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    use app::App;
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("error initializing log");
    mount_to_body(App);
}

#[cfg(feature = "ssr")]
fn shell(options: leptos::config::LeptosOptions) -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

    provide_meta_context();

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <Title text="Leptos App"/>
                <Stylesheet id="leptos" href="/pkg/leptos_start.css"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
                <MetaTags/>
            </head>
            <body>
                <leptos::ssr::AppShell options=options>
                    <app::App/>
                </leptos::ssr::AppShell>
            </body>
        </html>
    }
}
"#;
        std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;

        // App component for fullstack
        let app_rs = r#"use leptos::prelude::*;
use leptos_router::{components::*, path};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav class="navbar">
                <A href="/">"Home"</A>
                <A href="/about">"About"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/about") view=AboutPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Count: " {count}
        </button>
        <p>"This is a fullstack Leptos app with server-side rendering."</p>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <h1>"About"</h1>
        <p>"This is a Leptos fullstack app created with leptos init."</p>
    }
}
"#;
        std::fs::write(self.target_path.join("src/app.rs"), app_rs)?;

        Ok(())
    }

    fn generate_static_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Similar to fullstack but with static generation focus
        let main_rs = r#"use leptos::prelude::*;

mod app;
use app::App;

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("error initializing log");

    mount_to_body(App)
}
"#;
        std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;

        let app_rs = r#"use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="container">
            <h1>"Static Leptos Site"</h1>
            <p>"This is a static site generated with Leptos."</p>
        </div>
    }
}
"#;
        std::fs::write(self.target_path.join("src/app.rs"), app_rs)?;

        Ok(())
    }

    fn generate_api_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        let main_rs = match self.config.server {
            ServerBackend::Axum => r#"use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    data: Option<serde_json::Value>,
}

async fn health() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "API is healthy".to_string(),
        data: None,
    })
}

async fn echo(Json(payload): Json<serde_json::Value>) -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "Echo response".to_string(),
        data: Some(payload),
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/echo", post(echo))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
        
    println!("API server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
"#,
            _ => "// Other server backends not yet implemented\nfn main() { println!(\"API server\"); }",
        };

        std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;
        Ok(())
    }

    fn generate_assets(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.template != ProjectTemplate::Api {
            // Basic index.html for client-side apps
            let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Leptos App</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>
"#;
            std::fs::write(self.target_path.join("public/index.html"), index_html)?;

            // Basic styles
            if self.config.styling == Styling::Tailwind {
                let tailwind_css = r#"@tailwind base;
@tailwind components;
@tailwind utilities;

.navbar {
    @apply flex space-x-4 p-4 bg-gray-100;
}

.container {
    @apply max-w-4xl mx-auto p-4;
}
"#;
                std::fs::write(self.target_path.join("src/styles/tailwind.css"), tailwind_css)?;
            }
        }

        Ok(())
    }

    fn generate_configuration_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Generate README
        let readme = format!(
            r#"# {}

A Leptos {} application created with `leptos init`.

## Development

```bash
# Install cargo-leptos
cargo install cargo-leptos

# Start development server
cargo leptos watch

# Build for production
cargo leptos build --release
```

## Features

- Template: {:?}
- Server: {:?}
- Database: {:?}
- Styling: {:?}

Generated with Leptos Enhanced Initialization System.
"#,
            self.config.name,
            match self.config.template {
                ProjectTemplate::Spa => "SPA",
                ProjectTemplate::Fullstack => "fullstack",
                ProjectTemplate::Static => "static",
                ProjectTemplate::Api => "API",
                ProjectTemplate::Custom => "custom",
            },
            self.config.template,
            self.config.server,
            self.config.database,
            self.config.styling
        );

        std::fs::write(self.target_path.join("README.md"), readme)?;

        // Generate .gitignore
        let gitignore = r#"/target
/Cargo.lock
/dist
/pkg
/wasm-pack.log
/node_modules
/.env
/target/site
"#;
        std::fs::write(self.target_path.join(".gitignore"), gitignore)?;

        Ok(())
    }

    fn setup_validation_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Add validation dependencies to Cargo.toml
        let cargo_toml_path = self.target_path.join("Cargo.toml");
        let mut cargo_content = std::fs::read_to_string(&cargo_toml_path)?;

        // Add validation dependencies if not already present
        if !cargo_content.contains("leptos_compile_validator") {
            cargo_content.push_str(&format!(
                r#"

[build-dependencies]
leptos_compile_validator = {{ version = "0.1.0", features = ["build"] }}
"#
            ));
        }

        std::fs::write(&cargo_toml_path, cargo_content)?;

        // Create build.rs for compile-time validation
        let build_rs_content = r#"use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTOS_MODE");
    println!("cargo:rerun-if-env-changed=LEPTOS_TARGET");
    
    // Set build-time environment variables
    if let Ok(features) = env::var("CARGO_CFG_FEATURE") {
        println!("cargo:rustc-env=LEPTOS_FEATURES={}", features);
    }
    
    if let Ok(target) = env::var("TARGET") {
        if target.contains("wasm") {
            println!("cargo:rustc-env=LEPTOS_TARGET=client");
        } else {
            println!("cargo:rustc-env=LEPTOS_TARGET=server");
        }
    }
    
    // Set mode based on template
    match env::var("LEPTOS_TEMPLATE").as_deref() {
        Ok("spa") => println!("cargo:rustc-env=LEPTOS_MODE=spa"),
        Ok("fullstack") => println!("cargo:rustc-env=LEPTOS_MODE=fullstack"),
        Ok("static") => println!("cargo:rustc-env=LEPTOS_MODE=static"),
        Ok("api") => println!("cargo:rustc-env=LEPTOS_MODE=api"),
        _ => println!("cargo:rustc-env=LEPTOS_MODE=fullstack"),
    }
}
"#;

        std::fs::write(self.target_path.join("build.rs"), build_rs_content)?;

        // Create validation example in source
        let validation_example = format!(
            r#"// Compile-time validation examples
// 
// #[server_only]
// async fn database_query() -> Result<Data, Error> {{
//     // This function can only be used in server builds
//     // Will cause compile error if used in client build
// }}
//
// #[client_only] 
// fn local_storage_access() {{
//     // This function can only be used in client builds
//     // Will cause compile error if used in server build
// }}

// Template: {:?}
// Features will be automatically resolved based on build target
"#,
            self.config.template
        );

        std::fs::write(
            self.target_path.join("src/validation_examples.rs"),
            validation_example,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_spa_config_generation() {
        let config = InitConfig::for_template("test-spa".to_string(), ProjectTemplate::Spa);
        assert_eq!(config.client_features(), vec!["csr"]);
        assert_eq!(config.server_features(), Vec::<String>::new());
    }

    #[test]
    fn test_fullstack_config_generation() {
        let config = InitConfig::for_template("test-fullstack".to_string(), ProjectTemplate::Fullstack);
        assert_eq!(config.client_features(), vec!["hydrate"]);
        assert_eq!(config.server_features(), vec!["ssr"]);
    }

    #[test]
    fn test_project_generation() {
        let temp_dir = TempDir::new().unwrap();
        let config = InitConfig::for_template("test-project".to_string(), ProjectTemplate::Spa);
        let generator = ProjectGenerator::new(config, temp_dir.path().join("test-project"));

        generator.generate().expect("Project generation should succeed");

        // Verify key files exist
        assert!(temp_dir.path().join("test-project/Cargo.toml").exists());
        assert!(temp_dir.path().join("test-project/src/main.rs").exists());
        assert!(temp_dir.path().join("test-project/src/app.rs").exists());
        assert!(temp_dir.path().join("test-project/README.md").exists());
    }

    #[test]
    fn test_cargo_toml_generation() {
        let config = InitConfig::for_template("test-toml".to_string(), ProjectTemplate::Fullstack);
        let generator = ProjectGenerator::new(config, ".");
        let cargo_toml = generator.build_cargo_toml();

        assert!(cargo_toml.contains("name = \"test-toml\""));
        assert!(cargo_toml.contains("[features]"));
        assert!(cargo_toml.contains("leptos ="));
        assert!(cargo_toml.contains("[package.metadata.leptos]"));
    }
}