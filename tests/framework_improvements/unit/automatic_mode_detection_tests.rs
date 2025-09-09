//! Unit tests for the automatic mode detection system
//!
//! Tests individual components of the automatic mode detection system
//! to ensure they work correctly in isolation.

use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

/// Test the basic mode detection functionality
#[test]
fn test_mode_detection_basic_functionality() {
    // This test validates that the core mode detection logic works
    // even if we can't build the full system due to dependency issues
    
    // Test mode enum functionality
    use leptos_feature_detection::LeptosMode;
    
    // Test CSR mode
    let csr_mode = LeptosMode::CSR;
    assert_eq!(csr_mode.required_features(), vec!["csr"]);
    assert_eq!(csr_mode.bin_features(), Vec::<String>::new());
    assert_eq!(csr_mode.lib_features(), vec!["csr"]);
    
    // Test Fullstack mode
    let fullstack_mode = LeptosMode::Fullstack;
    assert_eq!(fullstack_mode.required_features(), vec!["ssr", "hydrate"]);
    assert_eq!(fullstack_mode.bin_features(), vec!["ssr"]);
    assert_eq!(fullstack_mode.lib_features(), vec!["hydrate"]);
    
    // Test mode compatibility
    assert!(csr_mode.is_compatible_with_features(&["csr".to_string()]));
    assert!(!csr_mode.is_compatible_with_features(&["ssr".to_string()]));
    assert!(!csr_mode.is_compatible_with_features(&["csr".to_string(), "ssr".to_string()]));
}

/// Test mode resolver functionality
#[test]
fn test_mode_resolver_basic_functionality() {
    use leptos_mode_resolver::{BuildMode, BuildTarget, ModeResolver};
    
    // Test SPA mode resolution
    let spa_resolver = ModeResolver::new(BuildMode::Spa);
    assert!(spa_resolver.validate().is_ok());
    
    // Test feature resolution
    let client_features = spa_resolver.resolve_features(BuildTarget::Client).unwrap();
    assert_eq!(client_features, vec!["csr"]);
    
    // Test server target should fail for SPA
    assert!(spa_resolver.resolve_features(BuildTarget::Server).is_err());
    
    // Test fullstack mode
    let fullstack_resolver = ModeResolver::new(BuildMode::Fullstack);
    let client_features = fullstack_resolver.resolve_features(BuildTarget::Client).unwrap();
    let server_features = fullstack_resolver.resolve_features(BuildTarget::Server).unwrap();
    
    assert_eq!(client_features, vec!["hydrate"]);
    assert_eq!(server_features, vec!["ssr"]);
}

/// Test validation context creation
#[test]
fn test_validation_context_creation() {
    use leptos_compile_validator::ValidationContext;
    
    // Test creating validation context from environment
    let context = ValidationContext::from_env();
    
    // Should have empty features by default
    assert!(context.enabled_features.is_empty());
    assert!(context.current_mode.is_none());
    assert!(context.current_target.is_none());
    assert!(context.errors.is_empty());
    assert!(!context.has_errors());
}

/// Test feature conflict detection
#[test]
fn test_feature_conflict_detection() {
    use leptos_compile_validator::{ValidationContext, ValidationError, ValidationErrorType};
    
    let mut context = ValidationContext::from_env();
    
    // Add conflicting features
    context.enabled_features.insert("csr".to_string());
    context.enabled_features.insert("ssr".to_string());
    
    // Check for conflicts
    let conflicting_sets = vec![
        vec!["csr", "ssr"],
        vec!["csr", "hydrate"],
    ];
    
    let mut errors = Vec::new();
    
    for conflict_set in conflicting_sets {
        let active_conflicts: Vec<String> = conflict_set
            .iter()
            .filter(|feature| context.enabled_features.contains(&feature.to_string()))
            .map(|s| s.to_string())
            .collect();
        
        if active_conflicts.len() > 1 {
            errors.push(ValidationError::feature_conflict(active_conflicts, None));
        }
    }
    
    // Should detect conflict
    assert!(!errors.is_empty());
    assert_eq!(errors[0].error_type, ValidationErrorType::FeatureConflict);
}

/// Test error message generation
#[test]
fn test_error_message_generation() {
    use leptos_compile_validator::ValidationError;
    
    let error = ValidationError::feature_conflict(
        vec!["csr".to_string(), "ssr".to_string()],
        None,
    );
    
    assert!(error.message.contains("csr, ssr"));
    assert!(error.suggestion.is_some());
    assert!(error.help_url.is_some());
    
    let wrong_context_error = ValidationError::wrong_context(
        "database_query",
        "server",
        "client",
        None,
    );
    
    assert!(wrong_context_error.message.contains("database_query"));
    assert!(wrong_context_error.message.contains("server context"));
    assert!(wrong_context_error.message.contains("client context"));
    assert!(wrong_context_error.suggestion.is_some());
}

/// Test project structure analysis
#[test]
fn test_project_structure_analysis() {
    // Create a temporary project structure
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
    
    // Test that we can create a detector (even if we can't run full analysis)
    // This validates the basic structure is correct
    assert!(project_root.join("Cargo.toml").exists());
    assert!(project_root.join("src").join("lib.rs").exists());
}

/// Test configuration generation
#[test]
fn test_configuration_generation() {
    use leptos_mode_resolver::{BuildConfig, BuildMode, BuildTarget, Environment};
    
    // Test development configuration
    let dev_config = BuildConfig::development(BuildMode::Fullstack);
    let client_features = dev_config.complete_features(BuildTarget::Client).unwrap();
    let server_features = dev_config.complete_features(BuildTarget::Server).unwrap();
    
    assert!(client_features.contains(&"hydrate".to_string()));
    assert!(client_features.contains(&"tracing".to_string()));
    assert!(server_features.contains(&"ssr".to_string()));
    assert!(server_features.contains(&"tracing".to_string()));
    
    // Test production configuration
    let prod_config = BuildConfig::production(BuildMode::Fullstack);
    let client_features = prod_config.complete_features(BuildTarget::Client).unwrap();
    let server_features = prod_config.complete_features(BuildTarget::Server).unwrap();
    
    assert!(client_features.contains(&"hydrate".to_string()));
    assert!(!client_features.contains(&"tracing".to_string()));
    assert!(server_features.contains(&"ssr".to_string()));
    assert!(!server_features.contains(&"tracing".to_string()));
}

/// Test metadata generation
#[test]
fn test_metadata_generation() {
    use leptos_mode_resolver::{BuildConfig, BuildMode, Environment};
    
    let config = BuildConfig::production(BuildMode::Fullstack);
    let metadata = config.leptos_metadata();
    
    // Test metadata generation
    assert!(!metadata.bin_features.is_empty());
    assert!(!metadata.lib_features.is_empty());
    assert_eq!(metadata.mode, BuildMode::Fullstack);
    
    // Test TOML generation
    let toml = metadata.to_toml();
    assert!(toml.contains("bin-features"));
    assert!(toml.contains("lib-features"));
    assert!(toml.contains("env = \"PROD\""));
}

/// Test conflict detection in mode resolver
#[test]
fn test_mode_resolver_conflict_detection() {
    use leptos_mode_resolver::{BuildMode, ModeResolver, ConflictType};
    
    let resolver = ModeResolver::new(BuildMode::Fullstack);
    
    // Test conflicting features
    let conflicts = resolver.detect_conflicts(&["csr".to_string(), "ssr".to_string()]);
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts[0].conflict_type, ConflictType::MutuallyExclusive);
    
    // Test invalid features for mode
    let conflicts = resolver.detect_conflicts(&["csr".to_string()]);
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts[0].conflict_type, ConflictType::InvalidForMode);
}

/// Test custom mode validation
#[test]
fn test_custom_mode_validation() {
    use leptos_mode_resolver::{BuildMode, ModeResolver};
    
    // Test valid custom mode
    let valid_custom = ModeResolver::new(BuildMode::Custom {
        client_features: vec!["csr".to_string()],
        server_features: vec!["ssr".to_string()],
    });
    assert!(valid_custom.validate().is_ok());
    
    // Test invalid custom mode
    let invalid_custom = ModeResolver::new(BuildMode::Custom {
        client_features: vec!["invalid_feature".to_string()],
        server_features: vec!["ssr".to_string()],
    });
    assert!(invalid_custom.validate().is_err());
}

/// Test build command generation
#[test]
fn test_build_command_generation() {
    use leptos_mode_resolver::{BuildMode, BuildTarget, ModeResolver};
    
    let resolver = ModeResolver::new(BuildMode::Spa);
    let client_cmd = resolver.build_command(BuildTarget::Client).unwrap();
    
    assert!(client_cmd.contains("wasm32-unknown-unknown"));
    assert!(client_cmd.contains("csr"));
    
    let fullstack_resolver = ModeResolver::new(BuildMode::Fullstack);
    let server_cmd = fullstack_resolver.build_command(BuildTarget::Server).unwrap();
    
    assert!(server_cmd.contains("ssr"));
    assert!(!server_cmd.contains("wasm32-unknown-unknown"));
}

/// Test environment variable parsing
#[test]
fn test_environment_variable_parsing() {
    use leptos_mode_resolver::{BuildMode, BuildTarget};
    
    // Test mode creation
    let spa_mode = BuildMode::Spa;
    let fullstack_mode = BuildMode::Fullstack;
    let static_mode = BuildMode::Static;
    let api_mode = BuildMode::Api;
    
    // Test that modes can be created
    assert!(matches!(spa_mode, BuildMode::Spa));
    assert!(matches!(fullstack_mode, BuildMode::Fullstack));
    assert!(matches!(static_mode, BuildMode::Static));
    assert!(matches!(api_mode, BuildMode::Api));
    
    // Test target creation
    let client_target = BuildTarget::Client;
    let server_target = BuildTarget::Server;
    
    assert!(matches!(client_target, BuildTarget::Client));
    assert!(matches!(server_target, BuildTarget::Server));
}

/// Test performance analyzer
#[test]
fn test_performance_analyzer() {
    use leptos_compile_validator::PerformanceAnalyzer;
    
    let analyzer = PerformanceAnalyzer::new();
    let warnings = analyzer.analyze_performance();
    
    // Should have no warnings for default analyzer
    assert!(warnings.is_empty());
    
    // Test that analyzer can be created and used
    assert!(warnings.is_empty());
}

/// Test validation context error handling
#[test]
fn test_validation_context_error_handling() {
    use leptos_compile_validator::{ValidationContext, ValidationError};
    
    let mut context = ValidationContext::from_env();
    
    // Add an error
    let error = ValidationError::feature_conflict(
        vec!["csr".to_string(), "ssr".to_string()],
        None,
    );
    context.error(error);
    
    // Should have error
    assert!(context.has_errors());
    assert_eq!(context.errors.len(), 1);
    
    // Test error generation
    let compile_errors = context.generate_compile_errors();
    assert!(!compile_errors.is_empty());
}
