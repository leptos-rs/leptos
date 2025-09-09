//! Tests for the compile-time validation system

use leptos_compile_validator::*;
use leptos_mode_resolver::{BuildMode, BuildTarget, ModeResolver};
use std::collections::HashSet;

#[test]
fn test_validation_context_creation() {
    // Test creating validation context from environment
    let context = ValidationContext::from_env();
    
    // Should have empty features by default
    assert!(context.enabled_features.is_empty());
    assert!(context.current_mode.is_none());
    assert!(context.current_target.is_none());
    assert!(context.errors.is_empty());
}

#[test]
fn test_validation_context_with_features() {
    // Set up environment variables
    std::env::set_var("CARGO_FEATURE_SSR", "1");
    std::env::set_var("CARGO_FEATURE_HYDRATE", "1");
    std::env::set_var("LEPTOS_MODE", "fullstack");
    std::env::set_var("LEPTOS_TARGET", "server");
    
    let context = ValidationContext::from_env();
    
    // Should have detected features
    assert!(context.enabled_features.contains(&"ssr".to_string()));
    assert!(context.enabled_features.contains(&"hydrate".to_string()));
    
    // Should have detected mode and target
    assert_eq!(context.current_mode, Some(BuildMode::Fullstack));
    assert_eq!(context.current_target, Some(BuildTarget::Server));
}

#[test]
fn test_feature_conflict_detection() {
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

#[test]
fn test_mode_specific_validation() {
    let mut context = ValidationContext::from_env();
    context.current_mode = Some(BuildMode::Spa);
    context.enabled_features.insert("ssr".to_string());
    
    let resolver = ModeResolver::new(BuildMode::Spa);
    
    // Should detect invalid feature for SPA mode
    assert!(!resolver.is_feature_valid("ssr"));
    
    // Should be valid for SPA mode
    assert!(resolver.is_feature_valid("csr"));
}

#[test]
fn test_validation_error_messages() {
    let error = ValidationError::feature_conflict(
        vec!["csr".to_string(), "ssr".to_string()],
        None,
    );
    
    assert!(error.message.contains("csr, ssr"));
    assert!(error.suggestion.is_some());
    assert!(error.help_url.is_some());
}

#[test]
fn test_wrong_context_error() {
    let error = ValidationError::wrong_context(
        "database_query",
        "server",
        "client",
        None,
    );
    
    assert!(error.message.contains("database_query"));
    assert!(error.message.contains("server context"));
    assert!(error.message.contains("client context"));
    assert!(error.suggestion.is_some());
}

#[test]
fn test_invalid_feature_error() {
    let error = ValidationError::invalid_feature(
        "ssr",
        "SPA mode",
        None,
    );
    
    assert!(error.message.contains("ssr"));
    assert!(error.message.contains("SPA mode"));
    assert!(error.suggestion.is_some());
}

#[test]
fn test_validate_features_function() {
    // Set up environment for testing
    std::env::set_var("CARGO_FEATURE_CSR", "1");
    std::env::set_var("CARGO_FEATURE_SSR", "1");
    
    let validation_result = validate_features();
    
    // Should generate compile errors for conflicting features
    assert!(!validation_result.is_empty());
}

#[test]
fn test_validate_with_context_function() {
    // Set up environment for testing
    std::env::set_var("CARGO_FEATURE_CSR", "1");
    std::env::set_var("CARGO_FEATURE_SSR", "1");
    std::env::set_var("LEPTOS_MODE", "spa");
    
    let validation_result = validate_with_context();
    
    // Should generate compile errors for conflicting features and invalid mode
    assert!(!validation_result.is_empty());
}

#[test]
fn test_performance_analyzer() {
    let analyzer = PerformanceAnalyzer::new();
    let warnings = analyzer.analyze_performance();
    
    // Should have no warnings for default analyzer
    assert!(warnings.is_empty());
}

#[test]
fn test_performance_analyzer_with_high_counts() {
    let analyzer = PerformanceAnalyzer {
        signal_count: 1500,
        effect_count: 600,
        component_count: 150,
    };
    
    let warnings = analyzer.analyze_performance();
    
    // Should have warnings for high counts
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| w.contains("signal count")));
    assert!(warnings.iter().any(|w| w.contains("effect count")));
    assert!(warnings.iter().any(|w| w.contains("component count")));
}

#[test]
fn test_mode_resolver_validation() {
    let resolver = ModeResolver::new(BuildMode::Spa);
    
    // Should be valid by default
    assert!(resolver.validate().is_ok());
    
    // Test custom mode validation
    let custom_resolver = ModeResolver::new(BuildMode::Custom {
        client_features: vec!["csr".to_string()],
        server_features: vec!["ssr".to_string()],
    });
    
    assert!(custom_resolver.validate().is_ok());
    
    // Test invalid custom mode
    let invalid_resolver = ModeResolver::new(BuildMode::Custom {
        client_features: vec!["invalid_feature".to_string()],
        server_features: vec!["ssr".to_string()],
    });
    
    assert!(invalid_resolver.validate().is_err());
}

#[test]
fn test_mode_resolver_feature_resolution() {
    let resolver = ModeResolver::new(BuildMode::Spa);
    
    // Test client target
    let client_features = resolver.resolve_features(BuildTarget::Client).unwrap();
    assert_eq!(client_features, vec!["csr"]);
    
    // Test server target (should fail for SPA mode)
    assert!(resolver.resolve_features(BuildTarget::Server).is_err());
}

#[test]
fn test_mode_resolver_fullstack_mode() {
    let resolver = ModeResolver::new(BuildMode::Fullstack);
    
    // Test client target
    let client_features = resolver.resolve_features(BuildTarget::Client).unwrap();
    assert_eq!(client_features, vec!["hydrate"]);
    
    // Test server target
    let server_features = resolver.resolve_features(BuildTarget::Server).unwrap();
    assert_eq!(server_features, vec!["ssr"]);
}

#[test]
fn test_mode_resolver_conflict_detection() {
    let resolver = ModeResolver::new(BuildMode::Fullstack);
    
    // Test conflicting features
    let conflicts = resolver.detect_conflicts(&["csr".to_string(), "ssr".to_string()]);
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts[0].conflict_type, leptos_mode_resolver::ConflictType::MutuallyExclusive);
    
    // Test invalid features for mode
    let conflicts = resolver.detect_conflicts(&["csr".to_string()]);
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts[0].conflict_type, leptos_mode_resolver::ConflictType::InvalidForMode);
}

#[test]
fn test_build_config_generation() {
    let config = BuildConfig::development(BuildMode::Fullstack);
    
    // Test client features
    let client_features = config.complete_features(BuildTarget::Client).unwrap();
    assert!(client_features.contains(&"hydrate".to_string()));
    assert!(client_features.contains(&"tracing".to_string()));
    
    // Test server features
    let server_features = config.complete_features(BuildTarget::Server).unwrap();
    assert!(server_features.contains(&"ssr".to_string()));
    assert!(server_features.contains(&"tracing".to_string()));
}

#[test]
fn test_leptos_metadata_generation() {
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

#[test]
fn test_validation_context_error_handling() {
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

#[test]
fn test_validation_context_parse_mode() {
    // Test valid modes
    assert_eq!(ValidationContext::parse_mode("spa"), Some(BuildMode::Spa));
    assert_eq!(ValidationContext::parse_mode("fullstack"), Some(BuildMode::Fullstack));
    assert_eq!(ValidationContext::parse_mode("static"), Some(BuildMode::Static));
    assert_eq!(ValidationContext::parse_mode("api"), Some(BuildMode::Api));
    
    // Test invalid mode
    assert_eq!(ValidationContext::parse_mode("invalid"), None);
    
    // Test case insensitive
    assert_eq!(ValidationContext::parse_mode("SPA"), Some(BuildMode::Spa));
    assert_eq!(ValidationContext::parse_mode("FullStack"), Some(BuildMode::Fullstack));
}

#[test]
fn test_validation_context_parse_target() {
    // Test WASM target
    assert_eq!(ValidationContext::parse_target("wasm32-unknown-unknown"), Some(BuildTarget::Client));
    assert_eq!(ValidationContext::parse_target("wasm32-wasi"), Some(BuildTarget::Client));
    
    // Test server target
    assert_eq!(ValidationContext::parse_target("x86_64-unknown-linux-gnu"), Some(BuildTarget::Server));
    assert_eq!(ValidationContext::parse_target("aarch64-apple-darwin"), Some(BuildTarget::Server));
}
