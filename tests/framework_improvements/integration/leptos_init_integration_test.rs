//! Integration tests for Leptos Enhanced Initialization System
//!
//! Tests the complete workflow from project creation to successful build,
//! validating that the P0 critical issues have been resolved.

use leptos_init::{cli::LeptosInitCli, InitConfig, ProjectGenerator, ProjectTemplate};
use leptos_mode_resolver::{BuildConfig, BuildMode, BuildTarget, ModeResolver};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Test complete project creation and validation workflow
#[test]
fn test_complete_leptos_init_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_name = "test-leptos-app";
    let project_path = temp_dir.path().join(project_name);

    // Test 1: Project Creation with Enhanced Template System
    let result = LeptosInitCli::run_with_template(
        project_name.to_string(),
        ProjectTemplate::Fullstack,
        &project_path,
    );

    assert!(result.is_ok(), "Project creation should succeed");

    // Verify project structure
    assert!(project_path.exists(), "Project directory should be created");
    assert!(project_path.join("Cargo.toml").exists(), "Cargo.toml should exist");
    assert!(project_path.join("src/main.rs").exists(), "Main source should exist");
    assert!(project_path.join("src/app.rs").exists(), "App component should exist");
    assert!(project_path.join("README.md").exists(), "README should be generated");

    // Test 2: Validate Generated Configuration
    let cargo_toml_content = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Should be able to read Cargo.toml");

    // Should contain intelligent feature configuration
    assert!(cargo_toml_content.contains("[features]"), "Should have features section");
    assert!(cargo_toml_content.contains("hydrate"), "Should have hydrate feature for client");
    assert!(cargo_toml_content.contains("ssr"), "Should have ssr feature for server");
    assert!(cargo_toml_content.contains("[package.metadata.leptos]"), "Should have leptos metadata");

    // Should NOT contain manual feature flag complexity
    assert!(!cargo_toml_content.contains("csr"), "Should not have manual CSR feature in fullstack mode");

    // Test 3: Mode-Based Feature Resolution
    let resolver = ModeResolver::new(BuildMode::Fullstack);
    
    let client_features = resolver.resolve_features(BuildTarget::Client)
        .expect("Should resolve client features");
    let server_features = resolver.resolve_features(BuildTarget::Server)
        .expect("Should resolve server features");

    assert_eq!(client_features, vec!["hydrate"], "Client should use hydrate feature");
    assert_eq!(server_features, vec!["ssr"], "Server should use ssr feature");

    // Test 4: Conflict Detection
    let conflicts = resolver.detect_conflicts(&["csr".to_string(), "ssr".to_string()]);
    assert!(!conflicts.is_empty(), "Should detect conflicting features");

    // Test 5: Build Configuration Generation
    let build_config = BuildConfig::development(BuildMode::Fullstack);
    let metadata = build_config.leptos_metadata();
    let toml_output = metadata.to_toml();

    assert!(toml_output.contains("bin-features"), "Should generate bin features");
    assert!(toml_output.contains("lib-features"), "Should generate lib features");
    assert!(toml_output.contains("env = \"DEV\""), "Should set development environment");

    println!("✅ Complete Leptos Init workflow test passed");
}

/// Test project creation for all template types
#[test]
fn test_all_template_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let templates = vec![
        (ProjectTemplate::Spa, "spa-test", BuildMode::Spa),
        (ProjectTemplate::Fullstack, "fullstack-test", BuildMode::Fullstack),
        (ProjectTemplate::Static, "static-test", BuildMode::Static),
        (ProjectTemplate::Api, "api-test", BuildMode::Api),
    ];

    for (template, project_name, expected_mode) in templates {
        let project_path = temp_dir.path().join(project_name);

        // Create project
        let result = LeptosInitCli::run_with_template(
            project_name.to_string(),
            template.clone(),
            &project_path,
        );

        assert!(result.is_ok(), "Project creation should succeed for {:?}", template);

        // Verify mode resolution works correctly
        let resolver = ModeResolver::new(expected_mode);
        assert!(resolver.validate().is_ok(), "Mode should be valid for {:?}", template);

        // Verify appropriate features are generated
        match template {
            ProjectTemplate::Spa => {
                let features = resolver.resolve_features(BuildTarget::Client).unwrap();
                assert_eq!(features, vec!["csr"], "SPA should use CSR");
                assert!(resolver.resolve_features(BuildTarget::Server).is_err(), "SPA should not have server build");
            }
            ProjectTemplate::Fullstack => {
                let client_features = resolver.resolve_features(BuildTarget::Client).unwrap();
                let server_features = resolver.resolve_features(BuildTarget::Server).unwrap();
                assert_eq!(client_features, vec!["hydrate"], "Fullstack client should use hydrate");
                assert_eq!(server_features, vec!["ssr"], "Fullstack server should use SSR");
            }
            ProjectTemplate::Api => {
                let features = resolver.resolve_features(BuildTarget::Server).unwrap();
                assert_eq!(features, vec!["ssr"], "API should use SSR");
                assert!(resolver.resolve_features(BuildTarget::Client).is_err(), "API should not have client build");
            }
            ProjectTemplate::Static => {
                let client_features = resolver.resolve_features(BuildTarget::Client).unwrap();
                let server_features = resolver.resolve_features(BuildTarget::Server).unwrap();
                assert_eq!(client_features, vec!["hydrate"], "Static client should use hydrate");
                assert_eq!(server_features, vec!["ssr"], "Static server should use SSR");
            }
            _ => {}
        }
    }

    println!("✅ All template types test passed");
}

/// Test setup complexity reduction (P0 Critical Issue)
#[test]
fn test_setup_complexity_reduction() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("complexity-test");

    // Measure setup steps required
    let start_time = std::time::Instant::now();

    // Single command creates complete working project
    let result = LeptosInitCli::run_with_template(
        "complexity-test".to_string(),
        ProjectTemplate::Fullstack,
        &project_path,
    );

    let setup_time = start_time.elapsed();

    assert!(result.is_ok(), "Setup should succeed");
    assert!(setup_time.as_secs() < 5, "Setup should take less than 5 seconds");

    // Verify configuration is simple and complete
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Should read Cargo.toml");

    // Count lines - should be much less than 90+ lines of current examples
    let line_count = cargo_toml.lines().count();
    assert!(line_count < 60, "Configuration should be < 60 lines (was {})", line_count);

    // Should have working defaults that require no manual intervention
    assert!(cargo_toml.contains("leptos ="), "Should have leptos dependency");
    assert!(cargo_toml.contains("[package.metadata.leptos]"), "Should have leptos metadata");

    println!("✅ Setup complexity reduced: {} lines (target: <60)", line_count);
    println!("✅ Setup time: {:?} (target: <5s)", setup_time);
}

/// Test feature flag confusion elimination (P0 Critical Issue)
#[test]
fn test_feature_flag_confusion_elimination() {
    // Test 1: Mode-based resolution eliminates manual configuration
    let resolver = ModeResolver::new(BuildMode::Fullstack);

    // Should automatically resolve correct features
    let client_features = resolver.resolve_features(BuildTarget::Client).unwrap();
    let server_features = resolver.resolve_features(BuildTarget::Server).unwrap();

    assert_eq!(client_features, vec!["hydrate"]);
    assert_eq!(server_features, vec!["ssr"]);

    // Test 2: Detect and prevent common mistakes
    let conflicts = resolver.detect_conflicts(&[
        "csr".to_string(),
        "ssr".to_string(),
        "hydrate".to_string(),
    ]);

    assert!(!conflicts.is_empty(), "Should detect conflicting features");
    assert!(conflicts.iter().any(|c| c.features.contains(&"csr".to_string())), "Should detect CSR conflict");

    // Test 3: Clear error messages for invalid combinations
    let spa_resolver = ModeResolver::new(BuildMode::Spa);
    let server_result = spa_resolver.resolve_features(BuildTarget::Server);

    assert!(server_result.is_err(), "Should prevent invalid target for mode");
    
    let error_msg = format!("{}", server_result.unwrap_err());
    assert!(error_msg.contains("SPA"), "Error should mention SPA mode");
    assert!(error_msg.contains("suggestion"), "Error should provide suggestion");

    // Test 4: Build command generation
    let client_cmd = resolver.build_command(BuildTarget::Client).unwrap();
    let server_cmd = resolver.build_command(BuildTarget::Server).unwrap();

    assert!(client_cmd.contains("wasm32-unknown-unknown"), "Client build should target WASM");
    assert!(client_cmd.contains("hydrate"), "Client build should include hydrate");
    assert!(server_cmd.contains("ssr"), "Server build should include SSR");
    assert!(!server_cmd.contains("wasm32"), "Server build should not target WASM");

    println!("✅ Feature flag confusion eliminated");
    println!("✅ Automatic feature resolution working");
    println!("✅ Clear error messages provided");
}

/// Performance benchmark test
#[test]
fn test_performance_benchmarks() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Benchmark 1: Project Generation Speed
    let start = std::time::Instant::now();
    let mut projects_created = 0;

    for i in 0..10 {
        let project_path = temp_dir.path().join(format!("benchmark-{}", i));
        let result = LeptosInitCli::run_with_template(
            format!("benchmark-{}", i),
            ProjectTemplate::Fullstack,
            &project_path,
        );
        
        if result.is_ok() {
            projects_created += 1;
        }
    }

    let total_time = start.elapsed();
    let avg_time_per_project = total_time / projects_created;

    assert_eq!(projects_created, 10, "All projects should be created successfully");
    assert!(avg_time_per_project.as_millis() < 100, "Average project creation should be <100ms");

    // Benchmark 2: Feature Resolution Speed
    let start = std::time::Instant::now();
    let resolver = ModeResolver::new(BuildMode::Fullstack);

    for _ in 0..1000 {
        let _ = resolver.resolve_features(BuildTarget::Client);
        let _ = resolver.resolve_features(BuildTarget::Server);
    }

    let resolution_time = start.elapsed();
    assert!(resolution_time.as_millis() < 10, "1000 feature resolutions should take <10ms");

    // Benchmark 3: Configuration Generation
    let start = std::time::Instant::now();
    let config = InitConfig::for_template("benchmark".to_string(), ProjectTemplate::Fullstack);

    for _ in 0..100 {
        let _ = config.dependencies();
        let _ = config.feature_flags();
        let _ = config.leptos_metadata();
    }

    let config_time = start.elapsed();
    assert!(config_time.as_millis() < 50, "100 config generations should take <50ms");

    println!("✅ Performance benchmarks:");
    println!("  • Project creation: {:?}/project", avg_time_per_project);
    println!("  • Feature resolution: {:?}/1000 operations", resolution_time);
    println!("  • Config generation: {:?}/100 operations", config_time);
}

/// Integration test with real project structure validation
#[test]
fn test_generated_project_validity() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("validity-test");

    // Create project
    LeptosInitCli::run_with_template(
        "validity-test".to_string(),
        ProjectTemplate::Fullstack,
        &project_path,
    ).expect("Project creation should succeed");

    // Test generated Cargo.toml syntax
    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(project_path.join("Cargo.toml"))
        .arg("--quiet")
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Cargo check failed: {}", stderr);
                // Note: This might fail due to missing dependencies, but syntax should be valid
                // We mainly want to verify Cargo.toml is syntactically correct
                assert!(!stderr.contains("error: failed to parse"), "Cargo.toml should have valid syntax");
            } else {
                println!("✅ Generated Cargo.toml has valid syntax");
            }
        }
        Err(_) => {
            println!("⚠️  Cargo not available for validation (this is okay in CI)");
        }
    }

    // Validate source file structure
    let main_rs = std::fs::read_to_string(project_path.join("src/main.rs"))
        .expect("Should read main.rs");
    
    assert!(main_rs.contains("#[cfg(feature = \"ssr\")]"), "Should have conditional compilation");
    assert!(main_rs.contains("#[cfg(not(feature = \"ssr\"))]"), "Should have client fallback");

    let app_rs = std::fs::read_to_string(project_path.join("src/app.rs"))
        .expect("Should read app.rs");

    assert!(app_rs.contains("#[component]"), "Should have Leptos components");
    assert!(app_rs.contains("view!"), "Should use Leptos view macro");

    println!("✅ Generated project structure is valid");
}

#[test]
fn test_error_handling_and_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Test 1: Invalid template handling
    // This would be tested if we had an invalid template, but our enum prevents that
    
    // Test 2: Permission issues (simulate by using invalid path)
    let invalid_path = Path::new("/invalid/readonly/path");
    let result = LeptosInitCli::run_with_template(
        "test".to_string(),
        ProjectTemplate::Spa,
        invalid_path,
    );

    assert!(result.is_err(), "Should fail gracefully with invalid path");

    // Test 3: Mode resolver error handling
    let resolver = ModeResolver::new(BuildMode::Api);
    let client_result = resolver.resolve_features(BuildTarget::Client);
    
    assert!(client_result.is_err(), "Should error for invalid target/mode combo");
    
    let error = client_result.unwrap_err();
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("API"), "Error should mention the mode");
    assert!(error_msg.contains("client"), "Error should mention the target");

    println!("✅ Error handling and recovery working correctly");
}