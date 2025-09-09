//! Integration tests for the automatic mode detection system

use leptos_feature_detection::{detection::SmartDetector, LeptosMode};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to create a temporary project structure
fn create_temp_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create src directory
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
    
    temp_dir
}

#[test]
fn test_spa_mode_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    assert_eq!(analysis.detected_mode, LeptosMode::CSR);
    assert!(analysis.confidence > 0.7);
}

#[test]
fn test_fullstack_mode_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Create both main.rs and lib.rs for fullstack mode
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
        .fallback_file_and_image(&leptos_options.site_root, &leptos_options.site_pkg_dir, None)
        .with_state(leptos_options);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
"#;
    
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    assert_eq!(analysis.detected_mode, LeptosMode::Fullstack);
    assert!(analysis.confidence > 0.8);
}

#[test]
fn test_ssr_mode_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Create main.rs with server code
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
    
    fs::write(project_root.join("src").join("main.rs"), main_rs).unwrap();
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    assert_eq!(analysis.detected_mode, LeptosMode::SSR);
    assert!(analysis.confidence > 0.7);
}

#[test]
fn test_hydrate_mode_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Create lib.rs with hydration code
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    assert_eq!(analysis.detected_mode, LeptosMode::Hydrate);
    assert!(analysis.confidence > 0.7);
}

#[test]
fn test_server_function_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    // Should detect fullstack mode due to server functions
    assert_eq!(analysis.detected_mode, LeptosMode::Fullstack);
    assert!(analysis.confidence > 0.8);
}

#[test]
fn test_conflicting_features_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    // Should detect issues with conflicting features
    assert!(!analysis.issues.is_empty());
    assert!(analysis.issues.iter().any(|issue| {
        issue.message.contains("conflicting") || issue.message.contains("conflict")
    }));
}

#[test]
fn test_recommendations_generation() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Create Cargo.toml with manual feature configuration
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    // Should generate recommendations for mode declaration
    assert!(!analysis.recommendations.is_empty());
    assert!(analysis.recommendations.iter().any(|rec| {
        rec.action.contains("mode declaration") || rec.action.contains("automatic")
    }));
}

#[test]
fn test_empty_project_detection() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Remove the src directory to test empty project
    fs::remove_dir_all(project_root.join("src")).unwrap();
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    // Should default to CSR mode for empty projects
    assert_eq!(analysis.detected_mode, LeptosMode::CSR);
    assert!(analysis.confidence < 0.5); // Low confidence for empty project
}

#[test]
fn test_complex_project_structure() {
    let temp_dir = create_temp_project();
    let project_root = temp_dir.path();
    
    // Create complex project structure
    fs::create_dir_all(project_root.join("src").join("components")).unwrap();
    fs::create_dir_all(project_root.join("src").join("server")).unwrap();
    fs::create_dir_all(project_root.join("src").join("client")).unwrap();
    
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
    
    let detector = SmartDetector::new(project_root);
    let analysis = detector.analyze_comprehensive().unwrap();
    
    // Should detect fullstack mode due to complex structure
    assert_eq!(analysis.detected_mode, LeptosMode::Fullstack);
    assert!(analysis.confidence > 0.8);
}

#[test]
fn test_mode_compatibility() {
    use leptos_feature_detection::LeptosMode;
    
    // Test CSR mode compatibility
    let csr_mode = LeptosMode::CSR;
    assert!(csr_mode.is_compatible_with_features(&["csr".to_string()]));
    assert!(!csr_mode.is_compatible_with_features(&["ssr".to_string()]));
    assert!(!csr_mode.is_compatible_with_features(&["csr".to_string(), "ssr".to_string()]));
    
    // Test SSR mode compatibility
    let ssr_mode = LeptosMode::SSR;
    assert!(ssr_mode.is_compatible_with_features(&["ssr".to_string()]));
    assert!(!ssr_mode.is_compatible_with_features(&["csr".to_string()]));
    assert!(!ssr_mode.is_compatible_with_features(&["ssr".to_string(), "hydrate".to_string()]));
    
    // Test Fullstack mode compatibility
    let fullstack_mode = LeptosMode::Fullstack;
    assert!(fullstack_mode.is_compatible_with_features(&["ssr".to_string(), "hydrate".to_string()]));
    assert!(!fullstack_mode.is_compatible_with_features(&["csr".to_string()]));
}

#[test]
fn test_feature_requirements() {
    use leptos_feature_detection::LeptosMode;
    
    // Test CSR mode requirements
    let csr_mode = LeptosMode::CSR;
    assert_eq!(csr_mode.required_features(), vec!["csr"]);
    assert_eq!(csr_mode.bin_features(), vec![]);
    assert_eq!(csr_mode.lib_features(), vec!["csr"]);
    
    // Test SSR mode requirements
    let ssr_mode = LeptosMode::SSR;
    assert_eq!(ssr_mode.required_features(), vec!["ssr"]);
    assert_eq!(ssr_mode.bin_features(), vec!["ssr"]);
    assert_eq!(ssr_mode.lib_features(), vec!["ssr"]);
    
    // Test Fullstack mode requirements
    let fullstack_mode = LeptosMode::Fullstack;
    assert_eq!(fullstack_mode.required_features(), vec!["ssr", "hydrate"]);
    assert_eq!(fullstack_mode.bin_features(), vec!["ssr"]);
    assert_eq!(fullstack_mode.lib_features(), vec!["hydrate"]);
}
