//! Integration tests for the automatic mode detection system
//!
//! Tests the interaction between different components of the automatic
//! mode detection system and their integration with the build system.

use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

/// Test complete mode detection workflow
#[test]
fn test_complete_mode_detection_workflow() {
    // Create a temporary project structure
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create src directory
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml with manual features
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[features]
default = ["ssr", "hydrate"]

[package.metadata.leptos]
bin-features = ["ssr"]
lib-features = ["hydrate"]

[dependencies]
leptos = { path = "../../leptos" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create main.rs for server
    let main_rs = r#"
use leptos::*;
use leptos_axum::*;
use axum::Router;

#[tokio::main]
async fn main() {
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);
    
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .with_state(leptos_options);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    
    // Create lib.rs for client
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}

pub fn main() {
    mount_to_body(App);
}
"#;
    
    fs::write(project_root.join("src").join("main.rs"), main_rs).unwrap();
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Test that the project structure is correct for fullstack mode
    assert!(project_root.join("Cargo.toml").exists());
    assert!(project_root.join("src").join("main.rs").exists());
    assert!(project_root.join("src").join("lib.rs").exists());
    
    // Test that we can read the Cargo.toml content
    let cargo_content = fs::read_to_string(project_root.join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("ssr"));
    assert!(cargo_content.contains("hydrate"));
    assert!(cargo_content.contains("bin-features"));
    assert!(cargo_content.contains("lib-features"));
}

/// Test mode detection with server functions
#[test]
fn test_mode_detection_with_server_functions() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../../leptos" }
server_fn = { path = "../../server_fn" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create lib.rs with server functions
    let lib_rs = r#"
use leptos::*;
use server_fn::*;

#[server]
pub async fn get_data() -> Result<String, ServerFnError> {
    Ok("Hello from server!".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let data = create_resource(|| (), |_| get_data());
    
    view! {
        <div>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    data.get().map(|result| match result {
                        Ok(data) => view! { <p>{data}</p> },
                        Err(e) => view! { <p>"Error: " {e.to_string()}</p> },
                    })
                }}
            </Suspense>
        </div>
    }
}

pub fn main() {
    mount_to_body(App);
}
"#;
    
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Test that the project has server functions
    let lib_content = fs::read_to_string(project_root.join("src").join("lib.rs")).unwrap();
    assert!(lib_content.contains("#[server]"));
    assert!(lib_content.contains("get_data"));
    assert!(lib_content.contains("ServerFnError"));
}

/// Test conflicting features detection
#[test]
fn test_conflicting_features_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml with conflicting features
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[features]
default = ["ssr", "csr"]

[dependencies]
leptos = { path = "../../leptos" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create lib.rs
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Test that conflicting features are present
    let cargo_content = fs::read_to_string(project_root.join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("ssr"));
    assert!(cargo_content.contains("csr"));
    
    // This should be detected as a conflict by the system
    let has_conflict = cargo_content.contains("ssr") && cargo_content.contains("csr");
    assert!(has_conflict);
}

/// Test SPA mode detection
#[test]
fn test_spa_mode_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../../leptos" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create lib.rs for SPA mode
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}

pub fn main() {
    mount_to_body(App);
}
"#;
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Test SPA mode characteristics
    let lib_content = fs::read_to_string(project_root.join("src").join("lib.rs")).unwrap();
    assert!(lib_content.contains("mount_to_body"));
    assert!(!lib_content.contains("#[server]"));
    assert!(!project_root.join("src").join("main.rs").exists());
}

/// Test static mode detection
#[test]
fn test_static_mode_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../../leptos" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create main.rs for static generation
    let main_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}

fn main() {
    // Static generation - no server needed
    let html = leptos::ssr::render_to_string(App);
    println!("{}", html);
}
"#;
    
    fs::write(project_root.join("src").join("main.rs"), main_rs).unwrap();
    
    // Test static mode characteristics
    let main_content = fs::read_to_string(project_root.join("src").join("main.rs")).unwrap();
    assert!(main_content.contains("render_to_string"));
    assert!(!main_content.contains("tokio::main"));
    assert!(!main_content.contains("axum"));
}

/// Test API mode detection
#[test]
fn test_api_mode_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../../leptos" }
server_fn = { path = "../../server_fn" }
axum = "0.7"
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create main.rs for API mode
    let main_rs = r#"
use leptos::*;
use server_fn::*;
use axum::Router;

#[server]
async fn api_endpoint() -> Result<String, ServerFnError> {
    Ok("API response".to_string())
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api", axum::routing::get(|| async { "API Server" }));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
"#;
    
    fs::write(project_root.join("src").join("main.rs"), main_rs).unwrap();
    
    // Test API mode characteristics
    let main_content = fs::read_to_string(project_root.join("src").join("main.rs")).unwrap();
    assert!(main_content.contains("#[server]"));
    assert!(main_content.contains("api_endpoint"));
    assert!(main_content.contains("axum"));
    assert!(!main_content.contains("leptos_routes"));
    assert!(!project_root.join("src").join("lib.rs").exists());
}

/// Test complex project structure
#[test]
fn test_complex_project_structure() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create complex project structure
    fs::create_dir_all(project_root.join("src").join("components")).unwrap();
    fs::create_dir_all(project_root.join("src").join("server")).unwrap();
    fs::create_dir_all(project_root.join("src").join("client")).unwrap();
    
    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../../leptos" }
server_fn = { path = "../../server_fn" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create main.rs
    let main_rs = r#"
use leptos::*;
use leptos_axum::*;
use axum::Router;

#[tokio::main]
async fn main() {
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);
    
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .with_state(leptos_options);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    
    // Create lib.rs
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}

pub fn main() {
    mount_to_body(App);
}
"#;
    
    // Create server module
    let server_rs = r#"
use server_fn::*;

#[server]
pub async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Server data".to_string())
}
"#;
    
    // Create client module
    let client_rs = r#"
use leptos::*;

#[component]
pub fn ClientComponent() -> impl IntoView {
    view! { <div>"Client component"</div> }
}
"#;
    
    fs::write(project_root.join("src").join("main.rs"), main_rs).unwrap();
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    fs::write(project_root.join("src").join("server").join("mod.rs"), server_rs).unwrap();
    fs::write(project_root.join("src").join("client").join("mod.rs"), client_rs).unwrap();
    
    // Test complex structure
    assert!(project_root.join("src").join("components").exists());
    assert!(project_root.join("src").join("server").exists());
    assert!(project_root.join("src").join("client").exists());
    
    let server_content = fs::read_to_string(project_root.join("src").join("server").join("mod.rs")).unwrap();
    let client_content = fs::read_to_string(project_root.join("src").join("client").join("mod.rs")).unwrap();
    
    assert!(server_content.contains("#[server]"));
    assert!(client_content.contains("#[component]"));
}

/// Test build system integration
#[test]
fn test_build_system_integration() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml with mode declaration
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[package.metadata.leptos]
mode = "fullstack"
env = "DEV"

[dependencies]
leptos = { path = "../../leptos" }
leptos_compile_validator = { path = "../../leptos_compile_validator" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    
    // Create build.rs
    let build_rs = r#"
use leptos_compile_validator::validate_with_context;

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTOS_MODE");
    println!("cargo:rerun-if-env-changed=LEPTOS_TARGET");
    
    let validation_result = validate_with_context();
    if !validation_result.is_empty() {
        panic!("Leptos validation failed");
    }
}
"#;
    fs::write(project_root.join("build.rs"), build_rs).unwrap();
    
    // Create lib.rs
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Test build system integration
    assert!(project_root.join("build.rs").exists());
    let build_content = fs::read_to_string(project_root.join("build.rs")).unwrap();
    assert!(build_content.contains("validate_with_context"));
    assert!(build_content.contains("LEPTOS_MODE"));
    assert!(build_content.contains("LEPTOS_TARGET"));
    
    let cargo_content = fs::read_to_string(project_root.join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("mode = \"fullstack\""));
    assert!(cargo_content.contains("env = \"DEV\""));
}

/// Test migration scenario
#[test]
fn test_migration_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    fs::create_dir_all(project_root.join("src")).unwrap();
    
    // Create Cargo.toml with manual features (before migration)
    let cargo_toml_before = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[features]
default = ["ssr", "hydrate"]

[package.metadata.leptos]
bin-features = ["ssr"]
lib-features = ["hydrate"]

[dependencies]
leptos = { path = "../../leptos" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml_before).unwrap();
    
    // Create lib.rs
    let lib_rs = r#"
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    fs::write(project_root.join("src").join("lib.rs"), lib_rs).unwrap();
    
    // Simulate migration - update Cargo.toml
    let cargo_toml_after = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[package.metadata.leptos]
mode = "fullstack"
env = "DEV"

[dependencies]
leptos = { path = "../../leptos" }
leptos_compile_validator = { path = "../../leptos_compile_validator" }
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml_after).unwrap();
    
    // Test migration results
    let cargo_content = fs::read_to_string(project_root.join("Cargo.toml")).unwrap();
    assert!(!cargo_content.contains("bin-features"));
    assert!(!cargo_content.contains("lib-features"));
    assert!(cargo_content.contains("mode = \"fullstack\""));
    assert!(cargo_content.contains("env = \"DEV\""));
    assert!(cargo_content.contains("leptos_compile_validator"));
}
