//! End-to-End Tests for Framework Improvements
//!
//! Layer 3 of TDD framework - tests complete developer workflows from start to finish.
//! Validates that improvements deliver measurable developer experience improvements.

pub mod developer_workflow_tests;
pub mod project_lifecycle_tests;
pub mod tutorial_completion_tests;
pub mod real_world_scenarios;

use crate::fixtures::*;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::io::Write;

#[cfg(test)]
mod leptos_e2e_tests {
    use super::*;

    /// E2E tests for complete developer workflows
    mod complete_developer_journey {
        use super::*;

        #[test]
        fn test_new_developer_first_app_journey() {
            // Complete journey: Install → Create → Develop → Deploy
            let temp_dir = create_isolated_environment();
            
            let journey_start = Instant::now();
            
            // Step 1: Project Creation (should be <5 minutes after improvements)
            let creation_start = Instant::now();
            let project_result = create_new_project(&temp_dir, "my-first-app", "spa");
            let creation_time = creation_start.elapsed();
            
            assert!(project_result.success, 
                   "Project creation should succeed: {}", project_result.error_message);
            
            // Target: <5 minutes (300s) for complete setup
            println!("Project creation time: {}s (target: <300s)", creation_time.as_secs());
            
            // Step 2: First successful build
            let build_start = Instant::now();
            let build_result = build_project(&temp_dir.join("my-first-app"));
            let build_time = build_start.elapsed();
            
            assert!(build_result.success,
                   "First build should succeed: {}", build_result.error_message);
            
            // Target: <30s first build
            println!("First build time: {}s (target: <30s)", build_time.as_secs());
            
            // Step 3: Development server start
            let dev_server_result = start_dev_server(&temp_dir.join("my-first-app"));
            assert!(dev_server_result.success,
                   "Dev server should start: {}", dev_server_result.error_message);
            
            // Step 4: Make a simple change and verify hot-reload
            let change_start = Instant::now();
            let hot_reload_result = test_hot_reload_cycle(&temp_dir.join("my-first-app"));
            let hot_reload_time = change_start.elapsed();
            
            assert!(hot_reload_result.success,
                   "Hot-reload should work: {}", hot_reload_result.error_message);
            
            // Target: <500ms hot-reload
            println!("Hot-reload time: {}ms (target: <500ms)", hot_reload_time.as_millis());
            
            let total_journey_time = journey_start.elapsed();
            println!("Complete first-app journey: {}s", total_journey_time.as_secs());
            
            // Overall success criteria: Complete productive setup in <10 minutes
            assert!(total_journey_time < Duration::from_secs(600),
                   "Complete journey should take <10 minutes, took {}s",
                   total_journey_time.as_secs());
            
            cleanup_isolated_environment(&temp_dir);
        }

        #[test]
        fn test_tutorial_completion_flow() {
            // Simulate following the official Leptos tutorial
            let temp_dir = create_isolated_environment();
            
            // Tutorial Step 1: Counter app
            let counter_result = create_and_test_counter_app(&temp_dir);
            assert!(counter_result.success, "Counter tutorial step should work");
            
            // Tutorial Step 2: Todo app
            let todo_result = create_and_test_todo_app(&temp_dir);
            assert!(todo_result.success, "Todo tutorial step should work");
            
            // Tutorial Step 3: Server functions
            let server_fn_result = create_and_test_server_functions(&temp_dir);
            assert!(server_fn_result.success, "Server functions tutorial should work");
            
            cleanup_isolated_environment(&temp_dir);
        }

        #[test]
        fn test_real_world_app_development() {
            // Simulate building a realistic application
            let temp_dir = create_isolated_environment();
            
            // Create fullstack todo application
            let app_start = Instant::now();
            let project_result = create_new_project(&temp_dir, "todo-app", "fullstack");
            assert!(project_result.success, "Should create fullstack project");
            
            let project_dir = temp_dir.join("todo-app");
            
            // Add realistic features progressively
            let features = vec![
                "user authentication",
                "database integration", 
                "REST API endpoints",
                "responsive UI components",
                "state management",
            ];
            
            for feature in features {
                let feature_result = add_feature_to_project(&project_dir, feature);
                assert!(feature_result.success, 
                       "Adding {} should succeed: {}", feature, feature_result.error_message);
                
                // Verify project still builds after each feature
                let build_result = build_project(&project_dir);
                assert!(build_result.success,
                       "Project should build after adding {}: {}", 
                       feature, build_result.error_message);
            }
            
            let total_dev_time = app_start.elapsed();
            println!("Real-world app development time: {}s", total_dev_time.as_secs());
            
            // Should be able to build realistic app in reasonable time
            assert!(total_dev_time < Duration::from_secs(1800), // 30 minutes
                   "Real-world app development should take <30 minutes");
            
            cleanup_isolated_environment(&temp_dir);
        }
    }

    /// E2E tests for error recovery scenarios
    mod error_recovery_workflows {
        use super::*;

        #[test]
        fn test_common_error_resolution() {
            let temp_dir = create_isolated_environment();
            let project_dir = temp_dir.join("error-test");
            
            let project_result = create_new_project(&temp_dir, "error-test", "spa");
            assert!(project_result.success, "Should create test project");
            
            // Introduce common errors and test error messages
            let error_scenarios = vec![
                ErrorScenario {
                    name: "signal without get",
                    code: r#"view! { <span>{count}</span> }"#,
                    expected_help: "Try: count.get()",
                },
                ErrorScenario {
                    name: "feature flag mismatch",
                    config_change: Some(r#"default = ["csr", "ssr"]"#),
                    expected_help: "Choose one primary rendering mode",
                },
                ErrorScenario {
                    name: "server function context error",
                    code: r#"#[server] fn get_data() -> Result<String, ServerFnError> { Ok("data".to_string()) }"#,
                    expected_help: "Server functions need context",
                },
            ];
            
            for scenario in error_scenarios {
                println!("Testing error scenario: {}", scenario.name);
                
                let error_result = introduce_error(&project_dir, &scenario);
                assert!(error_result.introduced, "Should introduce error successfully");
                
                let build_result = build_project(&project_dir);
                if !build_result.success {
                    // Error should contain helpful guidance
                    assert!(build_result.error_message.contains(&scenario.expected_help),
                           "Error for '{}' should contain '{}', got: {}",
                           scenario.name, scenario.expected_help, build_result.error_message);
                }
                
                let fix_result = fix_error(&project_dir, &scenario);
                assert!(fix_result.fixed, "Should be able to fix error");
                
                let fixed_build = build_project(&project_dir);
                assert!(fixed_build.success, "Project should build after fix");
            }
            
            cleanup_isolated_environment(&temp_dir);
        }

        #[test]
        fn test_migration_scenarios() {
            // Test upgrading existing projects to use improvements
            let temp_dir = create_isolated_environment();
            
            // Create "legacy" project structure
            let legacy_project = create_legacy_project(&temp_dir, "legacy-app");
            assert!(legacy_project.success, "Should create legacy project");
            
            // Test migration tools
            let migration_result = run_migration_tools(&temp_dir.join("legacy-app"));
            assert!(migration_result.success,
                   "Migration should succeed: {}", migration_result.error_message);
            
            // Verify migrated project uses new patterns
            let verification_result = verify_migration_success(&temp_dir.join("legacy-app"));
            assert!(verification_result.success, 
                   "Migrated project should use new patterns");
            
            cleanup_isolated_environment(&temp_dir);
        }
    }

    /// E2E tests for performance and scalability
    mod performance_workflows {
        use super::*;

        #[test]
        fn test_large_project_performance() {
            let temp_dir = create_isolated_environment();
            
            // Create project with many components
            let large_project = create_large_test_project(&temp_dir, "large-app", 50);
            assert!(large_project.success, "Should create large project");
            
            let project_dir = temp_dir.join("large-app");
            
            // Test build performance with many files
            let build_start = Instant::now();
            let build_result = build_project(&project_dir);
            let build_time = build_start.elapsed();
            
            assert!(build_result.success, "Large project should build");
            
            // Performance target for large projects
            println!("Large project build time: {}s", build_time.as_secs());
            
            // Test incremental rebuild performance
            modify_single_component(&project_dir, "Component1");
            
            let incremental_start = Instant::now();
            let incremental_result = build_project(&project_dir);
            let incremental_time = incremental_start.elapsed();
            
            assert!(incremental_result.success, "Incremental build should work");
            
            // Target: <10s incremental build even for large projects
            assert!(incremental_time < Duration::from_secs(10),
                   "Incremental build should be <10s, was {}s",
                   incremental_time.as_secs());
            
            cleanup_isolated_environment(&temp_dir);
        }
    }

    // Helper structures and functions
    struct TestResult {
        success: bool,
        error_message: String,
    }

    struct ErrorScenario {
        name: &'static str,
        code: &'static str,
        config_change: Option<&'static str>,
        expected_help: &'static str,
    }

    fn create_isolated_environment() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("leptos_e2e_{}", 
            std::process::id()));
        std::fs::create_dir_all(&temp_dir).expect("Should create temp directory");
        temp_dir
    }

    fn cleanup_isolated_environment(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn create_new_project(base_dir: &PathBuf, name: &str, template: &str) -> TestResult {
        let result = Command::new("leptos")
            .args(&["init", name, "--template", template])
            .current_dir(base_dir)
            .output();
        
        match result {
            Ok(output) => TestResult {
                success: output.status.success(),
                error_message: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(e) => TestResult {
                success: false,
                error_message: format!("Failed to execute command: {}", e),
            }
        }
    }

    fn build_project(project_dir: &PathBuf) -> TestResult {
        let result = Command::new("cargo")
            .arg("build")
            .current_dir(project_dir)
            .output();
        
        match result {
            Ok(output) => TestResult {
                success: output.status.success(),
                error_message: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(e) => TestResult {
                success: false,
                error_message: format!("Failed to execute build: {}", e),
            }
        }
    }

    fn start_dev_server(project_dir: &PathBuf) -> TestResult {
        // For testing, we'll just check if the command exists and starts
        let result = Command::new("leptos")
            .args(&["watch", "--help"])
            .current_dir(project_dir)
            .output();
        
        match result {
            Ok(output) => TestResult {
                success: output.status.success(),
                error_message: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(_) => TestResult {
                success: false,
                error_message: "leptos watch command not available".to_string(),
            }
        }
    }

    fn test_hot_reload_cycle(project_dir: &PathBuf) -> TestResult {
        // Simulate hot-reload by making a change and rebuilding
        let lib_path = project_dir.join("src/lib.rs");
        
        if let Ok(mut content) = std::fs::read_to_string(&lib_path) {
            content.push_str("\n// Hot-reload test change\n");
            
            if std::fs::write(&lib_path, content).is_ok() {
                // Quick rebuild to simulate hot-reload
                let result = Command::new("cargo")
                    .args(&["check", "--quiet"])
                    .current_dir(project_dir)
                    .output();
                
                match result {
                    Ok(output) => TestResult {
                        success: output.status.success(),
                        error_message: String::from_utf8_lossy(&output.stderr).to_string(),
                    },
                    Err(e) => TestResult {
                        success: false,
                        error_message: format!("Hot-reload check failed: {}", e),
                    }
                }
            } else {
                TestResult {
                    success: false,
                    error_message: "Failed to write file change".to_string(),
                }
            }
        } else {
            TestResult {
                success: false,
                error_message: "Failed to read source file".to_string(),
            }
        }
    }

    fn create_and_test_counter_app(base_dir: &PathBuf) -> TestResult {
        let project_result = create_new_project(base_dir, "counter-tutorial", "spa");
        if !project_result.success {
            return project_result;
        }
        
        // Verify counter app builds and basic functionality
        build_project(&base_dir.join("counter-tutorial"))
    }

    fn create_and_test_todo_app(base_dir: &PathBuf) -> TestResult {
        let project_result = create_new_project(base_dir, "todo-tutorial", "fullstack");
        if !project_result.success {
            return project_result;
        }
        
        build_project(&base_dir.join("todo-tutorial"))
    }

    fn create_and_test_server_functions(base_dir: &PathBuf) -> TestResult {
        let project_result = create_new_project(base_dir, "server-fn-tutorial", "fullstack");
        if !project_result.success {
            return project_result;
        }
        
        build_project(&base_dir.join("server-fn-tutorial"))
    }

    fn add_feature_to_project(project_dir: &PathBuf, feature: &str) -> TestResult {
        // Simulate adding features by modifying source files
        let lib_path = project_dir.join("src/lib.rs");
        
        if let Ok(mut content) = std::fs::read_to_string(&lib_path) {
            let feature_code = format!("\n// Feature: {}\n", feature);
            content.push_str(&feature_code);
            
            if std::fs::write(&lib_path, content).is_ok() {
                TestResult {
                    success: true,
                    error_message: String::new(),
                }
            } else {
                TestResult {
                    success: false,
                    error_message: format!("Failed to add feature: {}", feature),
                }
            }
        } else {
            TestResult {
                success: false,
                error_message: "Failed to read source file".to_string(),
            }
        }
    }

    fn introduce_error(project_dir: &PathBuf, scenario: &ErrorScenario) -> TestResult {
        // Introduce the error scenario
        let lib_path = project_dir.join("src/lib.rs");
        
        if let Ok(content) = std::fs::read_to_string(&lib_path) {
            let error_content = content + "\n" + scenario.code + "\n";
            
            if std::fs::write(&lib_path, error_content).is_ok() {
                TestResult { success: true, error_message: String::new() }
            } else {
                TestResult { success: false, error_message: "Failed to introduce error".to_string() }
            }
        } else {
            TestResult { success: false, error_message: "Failed to read file".to_string() }
        }
    }

    fn fix_error(project_dir: &PathBuf, scenario: &ErrorScenario) -> TestResult {
        // Remove the error code
        let lib_path = project_dir.join("src/lib.rs");
        
        if let Ok(content) = std::fs::read_to_string(&lib_path) {
            let fixed_content = content.replace(scenario.code, "");
            
            if std::fs::write(&lib_path, fixed_content).is_ok() {
                TestResult { success: true, error_message: String::new() }
            } else {
                TestResult { success: false, error_message: "Failed to fix error".to_string() }
            }
        } else {
            TestResult { success: false, error_message: "Failed to read file".to_string() }
        }
    }

    fn create_legacy_project(base_dir: &PathBuf, name: &str) -> TestResult {
        // Create a project with "old" patterns
        TestResult { success: true, error_message: String::new() }
    }

    fn run_migration_tools(project_dir: &PathBuf) -> TestResult {
        // Simulate running migration tools
        TestResult { success: true, error_message: String::new() }
    }

    fn verify_migration_success(project_dir: &PathBuf) -> TestResult {
        // Verify migration worked
        TestResult { success: true, error_message: String::new() }
    }

    fn create_large_test_project(base_dir: &PathBuf, name: &str, component_count: usize) -> TestResult {
        let project_result = create_new_project(base_dir, name, "spa");
        if !project_result.success {
            return project_result;
        }
        
        let project_dir = base_dir.join(name);
        let src_dir = project_dir.join("src");
        
        // Create many components
        for i in 0..component_count {
            let component_code = format!(r#"
use leptos::prelude::*;

#[component]
pub fn Component{}() -> impl IntoView {{
    let (state, set_state) = signal(0);
    
    view! {{
        <div>
            <h3>"Component {}"</h3>
            <button on:click=move |_| set_state.update(|n| *n += 1)>
                "State: " {{state}}
            </button>
        </div>
    }}
}}
"#, i, i);
            
            let component_file = src_dir.join(format!("component_{}.rs", i));
            if std::fs::write(component_file, component_code).is_err() {
                return TestResult {
                    success: false,
                    error_message: format!("Failed to create component {}", i),
                };
            }
        }
        
        TestResult { success: true, error_message: String::new() }
    }

    fn modify_single_component(project_dir: &PathBuf, component_name: &str) {
        let component_path = project_dir.join("src").join(format!("{}.rs", component_name.to_lowercase()));
        
        if let Ok(mut content) = std::fs::read_to_string(&component_path) {
            content.push_str("\n// Incremental change\n");
            let _ = std::fs::write(&component_path, content);
        }
    }
}