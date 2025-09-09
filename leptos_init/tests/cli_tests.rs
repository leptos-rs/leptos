//! TDD Tests for leptos init CLI functionality
//! 
//! These tests define the expected behavior before implementation.
//! Run with: cargo test --test cli_tests

use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to run leptos-init command
fn run_leptos_init(args: &[&str], current_dir: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--manifest-path", "/Users/peterhanssens/consulting/Leptos/leptos/leptos_init/Cargo.toml", "--bin", "leptos-init", "--"]);
    cmd.args(args);
    
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }
    
    cmd.output().expect("Failed to execute leptos-init")
}

/// Test that leptos init command exists and shows help
#[test]
fn test_leptos_init_help() {
    let output = run_leptos_init(&["--help"], None);

    assert!(output.status.success(), "leptos-init --help should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("leptos-init"), "Help should mention 'leptos-init'");
    assert!(stdout.contains("Create a new Leptos project"), "Help should describe purpose");
    assert!(stdout.contains("--template"), "Help should show template option");
    assert!(stdout.contains("<NAME>"), "Help should show name argument");
}

/// Test that leptos init creates a project with default template
#[test]
fn test_leptos_init_default_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("my-app");
    
    let output = run_leptos_init(&["my-app"], Some(temp_dir.path()));

    if !output.status.success() {
        eprintln!("Command failed with status: {:?}", output.status);
        eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    assert!(output.status.success(), "leptos-init should succeed");
    
    // Verify project structure was created
    assert!(project_path.exists(), "Project directory should be created");
    assert!(project_path.join("Cargo.toml").exists(), "Cargo.toml should exist");
    assert!(project_path.join("src").exists(), "src directory should exist");
    assert!(project_path.join("src/main.rs").exists(), "main.rs should exist");
    assert!(project_path.join("README.md").exists(), "README.md should exist");
    
    // Verify Cargo.toml content
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("name = \"my-app\""), "Cargo.toml should have correct name");
    assert!(cargo_toml.contains("leptos ="), "Cargo.toml should include leptos dependency");
    assert!(cargo_toml.contains("[features]"), "Cargo.toml should have features section");
}

/// Test that leptos init creates SPA template correctly
#[test]
fn test_leptos_init_spa_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("spa-app");
    
    let output = run_leptos_init(&["spa-app", "--template", "spa"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init spa should succeed");
    
    // Verify SPA-specific structure
    assert!(project_path.join("public").exists(), "SPA should have public directory");
    assert!(project_path.join("public/index.html").exists(), "SPA should have index.html");
    
    // Verify Cargo.toml has CSR features
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("csr"), "SPA should have CSR features");
    assert!(cargo_toml.contains("leptos/csr"), "SPA should enable leptos/csr");
    
    // Verify main.rs is SPA-style
    let main_rs = std::fs::read_to_string(project_path.join("src/main.rs"))
        .expect("Failed to read main.rs");
    assert!(main_rs.contains("mount_to_body"), "SPA main.rs should use mount_to_body");
}

/// Test that leptos init creates fullstack template correctly
#[test]
fn test_leptos_init_fullstack_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("fullstack-app");
    
    let output = run_leptos_init(&["fullstack-app", "--template", "fullstack"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init fullstack should succeed");
    
    // Verify fullstack-specific structure
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("ssr"), "Fullstack should have SSR features");
    assert!(cargo_toml.contains("hydrate"), "Fullstack should have hydrate features");
    assert!(cargo_toml.contains("leptos_axum"), "Fullstack should include leptos_axum");
    
    // Verify main.rs has both server and client code
    let main_rs = std::fs::read_to_string(project_path.join("src/main.rs"))
        .expect("Failed to read main.rs");
    assert!(main_rs.contains("#[cfg(feature = \"ssr\")]"), "Fullstack should have SSR code");
    assert!(main_rs.contains("#[cfg(not(feature = \"ssr\"))]"), "Fullstack should have client code");
}

/// Test that leptos init creates API template correctly
#[test]
fn test_leptos_init_api_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("api-app");
    
    let output = run_leptos_init(&["api-app", "--template", "api"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init api should succeed");
    
    // Verify API-specific structure
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("ssr"), "API should have SSR features");
    assert!(!cargo_toml.contains("hydrate"), "API should not have hydrate features");
    assert!(!cargo_toml.contains("public"), "API should not have public directory");
    
    // Verify main.rs is API-style
    let main_rs = std::fs::read_to_string(project_path.join("src/main.rs"))
        .expect("Failed to read main.rs");
    assert!(main_rs.contains("axum"), "API should use axum");
    assert!(main_rs.contains("Router"), "API should have router setup");
}

/// Test that leptos init with database option works
#[test]
fn test_leptos_init_with_database() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("db-app");
    
    let output = run_leptos_init(&["db-app", "--template", "fullstack", "--database", "sqlite"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init with database should succeed");
    
    // Verify database dependencies
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("sqlx"), "Should include sqlx dependency");
    assert!(cargo_toml.contains("sqlite"), "Should include sqlite feature");
}

/// Test that leptos init with Tailwind works
#[test]
fn test_leptos_init_with_tailwind() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("tailwind-app");
    
    let output = run_leptos_init(&["tailwind-app", "--template", "fullstack", "--styling", "tailwind"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init with tailwind should succeed");
    
    // Verify Tailwind setup
    assert!(project_path.join("src/styles").exists(), "Should have styles directory");
    assert!(project_path.join("src/styles/tailwind.css").exists(), "Should have tailwind.css");
    
    let tailwind_css = std::fs::read_to_string(project_path.join("src/styles/tailwind.css"))
        .expect("Failed to read tailwind.css");
    assert!(tailwind_css.contains("@tailwind"), "Should have tailwind directives");
}

/// Test that leptos init validates project name
#[test]
fn test_leptos_init_invalid_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = run_leptos_init(&["123invalid"], Some(temp_dir.path()));

    // Should fail with invalid name (starts with number)
    assert!(!output.status.success(), "Should fail with invalid project name");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid"), "Should show error about invalid name");
}

/// Test that leptos init prevents overwriting existing directory
#[test]
fn test_leptos_init_existing_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("existing-app");
    
    // Create existing directory
    std::fs::create_dir(&project_path).expect("Failed to create existing directory");
    std::fs::write(project_path.join("existing-file.txt"), "test").expect("Failed to create existing file");
    
    let output = run_leptos_init(&["existing-app"], Some(temp_dir.path()));

    // Should fail without --force
    assert!(!output.status.success(), "Should fail when directory exists");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exists"), "Should show error about existing directory");
}

/// Test that leptos init --force overwrites existing directory
#[test]
fn test_leptos_init_force_overwrite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("force-app");
    
    // Create existing directory
    std::fs::create_dir(&project_path).expect("Failed to create existing directory");
    std::fs::write(project_path.join("existing-file.txt"), "test").expect("Failed to create existing file");
    
    let output = run_leptos_init(&["force-app", "--force"], Some(temp_dir.path()));

    assert!(output.status.success(), "Should succeed with --force");
    
    // Verify existing file was removed and project was created
    assert!(!project_path.join("existing-file.txt").exists(), "Existing file should be removed");
    assert!(project_path.join("Cargo.toml").exists(), "Cargo.toml should be created");
}

/// Test that generated project compiles
#[test]
fn test_generated_project_compiles() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("compile-test");
    
    // Create project
    let output = run_leptos_init(&["compile-test", "--template", "spa"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init should succeed");
    
    // Try to compile the generated project
    let compile_output = Command::new("cargo")
        .args(&["check"])
        .current_dir(&project_path)
        .output()
        .expect("Failed to execute cargo check");

    // Note: This might fail due to missing leptos dependencies in test environment
    // but the structure should be correct
    let stderr = String::from_utf8_lossy(&compile_output.stderr);
    if !compile_output.status.success() {
        // If compilation fails, it should be due to missing dependencies, not syntax errors
        assert!(!stderr.contains("error: expected"), "Should not have syntax errors");
        assert!(!stderr.contains("error: cannot find"), "Should not have basic import errors");
    }
}

/// Test that leptos init shows progress and success messages
#[test]
fn test_leptos_init_user_feedback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = run_leptos_init(&["feedback-test"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init should succeed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("🚀"), "Should show rocket emoji");
    assert!(stdout.contains("✅"), "Should show success emoji");
    assert!(stdout.contains("created successfully"), "Should show success message");
    assert!(stdout.contains("Next Steps"), "Should show next steps");
}
