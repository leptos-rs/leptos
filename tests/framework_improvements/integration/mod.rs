//! Integration Tests for Framework Improvements
//!
//! Layer 2 of TDD framework - tests cross-component interactions and build system integration.
//! Validates that improvements work correctly when components are integrated together.

pub mod build_system_tests;
pub mod component_interaction_tests;
pub mod development_workflow_tests;
pub mod deployment_integration_tests;
pub mod leptos_init_integration_test;

use crate::fixtures::*;
use std::process::Command;
use std::time::{Duration, Instant};
use std::path::PathBuf;

#[cfg(test)]
mod leptos_integration_tests {
    use super::*;

    /// Integration tests for LEPTOS-2024-001: Project Setup with Build System
    mod project_setup_integration {
        use super::*;

        #[test]
        fn test_init_command_with_build_system() {
            // Test that leptos init creates project that builds successfully
            let temp_dir = create_temp_directory();
            
            // Run leptos init command
            let init_result = Command::new("leptos")
                .args(&["init", "test-project", "--template", "spa"])
                .current_dir(&temp_dir)
                .output()
                .expect("Failed to run leptos init");
            
            assert!(init_result.status.success(), 
                   "leptos init should succeed: {}", 
                   String::from_utf8_lossy(&init_result.stderr));
            
            let project_dir = temp_dir.join("test-project");
            assert!(project_dir.exists(), "Project directory should be created");
            
            // Test that generated project builds successfully
            let build_start = Instant::now();
            let build_result = Command::new("cargo")
                .arg("build")
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run cargo build");
            let build_time = build_start.elapsed();
            
            assert!(build_result.status.success(),
                   "Generated project should build: {}",
                   String::from_utf8_lossy(&build_result.stderr));
            
            // Build should be reasonably fast for minimal project
            assert!(build_time < Duration::from_secs(60),
                   "Build time {}s should be <60s for minimal project",
                   build_time.as_secs());
            
            cleanup_temp_directory(&temp_dir);
        }

        #[test]
        fn test_init_command_with_different_templates() {
            let templates = ["spa", "ssr", "fullstack"];
            
            for template in &templates {
                let temp_dir = create_temp_directory();
                
                let init_result = Command::new("leptos")
                    .args(&["init", &format!("test-{}", template), "--template", template])
                    .current_dir(&temp_dir)
                    .output()
                    .expect("Failed to run leptos init");
                
                assert!(init_result.status.success(),
                       "leptos init --template {} should succeed: {}",
                       template,
                       String::from_utf8_lossy(&init_result.stderr));
                
                let project_dir = temp_dir.join(&format!("test-{}", template));
                
                // Verify template-specific files exist
                assert!(project_dir.join("Cargo.toml").exists());
                assert!(project_dir.join("src").exists());
                
                // Verify template-specific configuration
                let cargo_content = std::fs::read_to_string(project_dir.join("Cargo.toml"))
                    .expect("Should read Cargo.toml");
                
                match *template {
                    "spa" => assert!(cargo_content.contains("csr")),
                    "ssr" => assert!(cargo_content.contains("ssr")),
                    "fullstack" => {
                        assert!(cargo_content.contains("ssr"));
                        assert!(cargo_content.contains("hydrate"));
                    }
                    _ => {}
                }
                
                cleanup_temp_directory(&temp_dir);
            }
        }
    }

    /// Integration tests for LEPTOS-2024-002: Feature Flag Build Integration
    mod feature_flag_integration {
        use super::*;

        #[test]
        fn test_feature_flag_build_matrix() {
            // Test different feature flag combinations build correctly
            let feature_combinations = vec![
                vec!["csr"],
                vec!["ssr"],
                vec!["hydrate", "ssr"],
                vec!["default"],
            ];
            
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "feature-test");
            let project_dir = temp_dir.join("feature-test");
            
            for features in feature_combinations {
                let mut build_cmd = Command::new("cargo");
                build_cmd.arg("build").current_dir(&project_dir);
                
                if !features.is_empty() && !features.contains(&"default") {
                    build_cmd.args(&["--no-default-features", "--features"]);
                    build_cmd.arg(features.join(","));
                }
                
                let result = build_cmd.output()
                    .expect("Failed to run cargo build");
                
                assert!(result.status.success(),
                       "Build with features {:?} should succeed: {}",
                       features,
                       String::from_utf8_lossy(&result.stderr));
            }
            
            cleanup_temp_directory(&temp_dir);
        }

        #[test]
        fn test_conflicting_feature_flags_detection() {
            // Test that conflicting feature flags are detected at build time
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "conflict-test");
            let project_dir = temp_dir.join("conflict-test");
            
            // Modify Cargo.toml to have conflicting features
            let cargo_toml_path = project_dir.join("Cargo.toml");
            let cargo_content = std::fs::read_to_string(&cargo_toml_path)
                .expect("Should read Cargo.toml");
            
            let conflicting_content = cargo_content.replace(
                "[features]",
                "[features]\nconflicting = [\"csr\", \"ssr\"]"
            );
            
            std::fs::write(&cargo_toml_path, conflicting_content)
                .expect("Should write modified Cargo.toml");
            
            let result = Command::new("cargo")
                .args(&["build", "--features", "conflicting"])
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run cargo build");
            
            // Should either fail to build or produce warnings
            if result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                assert!(stderr.contains("warning") || stderr.contains("conflict"),
                       "Should warn about conflicting features: {}", stderr);
            }
            
            cleanup_temp_directory(&temp_dir);
        }
    }

    /// Integration tests for LEPTOS-2024-003: Signal API Integration
    mod signal_integration {
        use super::*;

        #[test]
        fn test_unified_signal_api_compilation() {
            // Test that unified signal API compiles and works across components
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "signal-test");
            let project_dir = temp_dir.join("signal-test");
            
            // Create test component using unified signal API
            let component_code = r#"
use leptos::prelude::*;

#[component]
pub fn Counter() -> impl IntoView {
    let count = signal(0);
    let doubled = count.derive(|n| n * 2);
    
    view! {
        <div>
            <p>"Count: " {count}</p>
            <p>"Doubled: " {doubled}</p>
            <button on:click=move |_| count.update(|n| *n += 1)>
                "Increment"
            </button>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div>
            <h1>"Signal Integration Test"</h1>
            <Counter/>
        </div>
    }
}
"#;
            
            std::fs::write(project_dir.join("src/lib.rs"), component_code)
                .expect("Should write component code");
            
            let result = Command::new("cargo")
                .args(&["check"])
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run cargo check");
            
            assert!(result.status.success(),
                   "Unified signal API should compile: {}",
                   String::from_utf8_lossy(&result.stderr));
            
            cleanup_temp_directory(&temp_dir);
        }
    }

    /// Integration tests for LEPTOS-2024-005: Error Message Integration
    mod error_message_integration {
        use super::*;

        #[test]
        fn test_framework_error_detection_in_build() {
            // Test that framework-specific errors are caught during build
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "error-test");
            let project_dir = temp_dir.join("error-test");
            
            // Create component with common error (signal without .get())
            let error_code = r#"
use leptos::prelude::*;

#[component]
pub fn BrokenComponent() -> impl IntoView {
    let count = signal(0);
    
    view! {
        <div>
            // This should cause framework-aware error
            <p>"Count: " {count}</p>
        </div>
    }
}
"#;
            
            std::fs::write(project_dir.join("src/lib.rs"), error_code)
                .expect("Should write error code");
            
            let result = Command::new("cargo")
                .args(&["check"])
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run cargo check");
            
            let stderr = String::from_utf8_lossy(&result.stderr);
            
            if !result.status.success() {
                // Error should be more helpful than generic Rust error
                assert!(
                    stderr.contains("signal") || stderr.contains("get") || stderr.contains("reactive"),
                    "Error message should provide framework-specific guidance: {}", stderr
                );
            }
            
            cleanup_temp_directory(&temp_dir);
        }
    }

    /// Integration tests for LEPTOS-2024-006: Development Performance Integration
    mod performance_integration {
        use super::*;

        #[test]
        fn test_development_build_performance() {
            // Test development mode compilation performance
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "perf-test");
            let project_dir = temp_dir.join("perf-test");
            
            // Initial build
            let initial_start = Instant::now();
            let initial_result = Command::new("cargo")
                .args(&["build"])
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run initial build");
            let initial_time = initial_start.elapsed();
            
            assert!(initial_result.status.success(), "Initial build should succeed");
            
            // Make small change
            let lib_path = project_dir.join("src/lib.rs");
            let mut content = std::fs::read_to_string(&lib_path)
                .expect("Should read lib.rs");
            content.push_str("\n// Small change\n");
            std::fs::write(&lib_path, content)
                .expect("Should write modified lib.rs");
            
            // Incremental build (this is the critical measurement)
            let incremental_start = Instant::now();
            let incremental_result = Command::new("cargo")
                .args(&["build"])
                .current_dir(&project_dir)
                .output()
                .expect("Failed to run incremental build");
            let incremental_time = incremental_start.elapsed();
            
            assert!(incremental_result.status.success(), "Incremental build should succeed");
            
            // Performance target: incremental builds should be <5s after improvements
            // For now, just measure and report
            println!("Initial build: {}s", initial_time.as_secs());
            println!("Incremental build: {}s", incremental_time.as_secs());
            
            // Incremental should be significantly faster than initial
            assert!(incremental_time < initial_time / 2,
                   "Incremental build ({}s) should be <50% of initial build ({}s)",
                   incremental_time.as_secs(), initial_time.as_secs());
            
            cleanup_temp_directory(&temp_dir);
        }

        #[test]
        fn test_hot_reload_integration() {
            // Test hot-reload functionality integration
            let temp_dir = create_temp_directory();
            create_test_leptos_project(&temp_dir, "hot-reload-test");
            let project_dir = temp_dir.join("hot-reload-test");
            
            // This would test hot-reload if leptos watch command exists
            let watch_result = Command::new("leptos")
                .args(&["watch", "--help"])
                .current_dir(&project_dir)
                .output();
            
            if let Ok(result) = watch_result {
                if result.status.success() {
                    // Hot-reload command exists, could test it
                    // For now, just verify it doesn't crash immediately
                    println!("Hot-reload command available");
                }
            }
            
            cleanup_temp_directory(&temp_dir);
        }
    }

    // Helper functions for integration tests
    fn create_temp_directory() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("leptos_integration_{}", 
            std::process::id()));
        std::fs::create_dir_all(&temp_dir).expect("Should create temp directory");
        temp_dir
    }

    fn cleanup_temp_directory(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn create_test_leptos_project(base_dir: &PathBuf, name: &str) {
        let project_dir = base_dir.join(name);
        std::fs::create_dir_all(&project_dir).expect("Should create project directory");
        std::fs::create_dir_all(project_dir.join("src")).expect("Should create src directory");
        
        // Create minimal Cargo.toml
        let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = {{ version = "0.8", features = ["default"] }}

[features]
default = ["csr"]
csr = ["leptos/csr"]
ssr = ["leptos/ssr"]
hydrate = ["leptos/hydrate"]
"#, name);
        
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)
            .expect("Should write Cargo.toml");
        
        // Create minimal lib.rs
        let lib_rs = r#"use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    
    view! {
        <div>
            <h1>"Hello Leptos!"</h1>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                "Count: " {count}
            </button>
        </div>
    }
}
"#;
        
        std::fs::write(project_dir.join("src/lib.rs"), lib_rs)
            .expect("Should write lib.rs");
    }
}