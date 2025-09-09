//! Leptos Compile-Time Validation System
//!
//! Provides compile-time validation for mode/feature conflicts,
//! preventing runtime errors and configuration mistakes.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::HashSet;

pub use leptos_mode_resolver::{BuildMode, BuildTarget, ModeResolver, ModeResolverError};

/// Compile-time validation context
pub struct ValidationContext {
    pub current_mode: Option<BuildMode>,
    pub current_target: Option<BuildTarget>,
    pub enabled_features: HashSet<String>,
    pub errors: Vec<ValidationError>,
}

impl ValidationContext {
    /// Create new validation context from environment
    pub fn from_env() -> Self {
        let mut context = Self {
            current_mode: None,
            current_target: None,
            enabled_features: HashSet::new(),
            errors: Vec::new(),
        };

        // Read from environment variables set by cargo-leptos or build system
        if let Ok(mode) = std::env::var("LEPTOS_MODE") {
            context.current_mode = Self::parse_mode(&mode);
        }

        if let Ok(target) = std::env::var("LEPTOS_TARGET") {
            context.current_target = Self::parse_target(&target);
        }

        // Read enabled features from CARGO_FEATURE_*
        for (key, _) in std::env::vars() {
            if let Some(feature) = key.strip_prefix("CARGO_FEATURE_") {
                context.enabled_features.insert(feature.to_lowercase());
            }
        }

        context
    }

    fn parse_mode(mode_str: &str) -> Option<BuildMode> {
        match mode_str.to_lowercase().as_str() {
            "spa" => Some(BuildMode::Spa),
            "fullstack" => Some(BuildMode::Fullstack),
            "static" => Some(BuildMode::Static),
            "api" => Some(BuildMode::Api),
            _ => None,
        }
    }

    fn parse_target(target_str: &str) -> Option<BuildTarget> {
        if target_str.contains("wasm") {
            Some(BuildTarget::Client)
        } else {
            Some(BuildTarget::Server)
        }
    }

    /// Add validation error
    pub fn error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Check if context has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Generate compile errors for all validation failures
    pub fn generate_compile_errors(&self) -> TokenStream {
        if self.errors.is_empty() {
            return quote! {};
        }

        let error_tokens: Vec<TokenStream> = self.errors
            .iter()
            .map(|error| error.to_compile_error())
            .collect();

        quote! {
            #(#error_tokens)*
        }
    }
}

/// Validation error with actionable suggestions
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub suggestion: Option<String>,
    pub span: Option<Span>,
    pub help_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    /// Feature flag conflict (e.g., csr + ssr)
    FeatureConflict,
    /// Invalid feature for current mode
    InvalidFeature,
    /// Function used in wrong context (client vs server)
    WrongContext,
    /// Missing required feature
    MissingFeature,
    /// Deprecated usage
    Deprecated,
}

impl ValidationError {
    /// Create new feature conflict error
    pub fn feature_conflict(
        conflicting_features: Vec<String>,
        span: Option<Span>,
    ) -> Self {
        let features_str = conflicting_features.join(", ");
        Self {
            error_type: ValidationErrorType::FeatureConflict,
            message: format!(
                "Conflicting features detected: {}. These features are mutually exclusive.",
                features_str
            ),
            suggestion: Some(
                "Use mode-based configuration instead. See: https://leptos.dev/modes".to_string()
            ),
            span,
            help_url: Some("https://leptos.dev/troubleshooting/feature-conflicts".to_string()),
        }
    }

    /// Create new wrong context error
    pub fn wrong_context(
        function_name: &str,
        expected_context: &str,
        actual_context: &str,
        span: Option<Span>,
    ) -> Self {
        Self {
            error_type: ValidationErrorType::WrongContext,
            message: format!(
                "Function '{}' can only be used in {} context, but this is {} context",
                function_name, expected_context, actual_context
            ),
            suggestion: Some(Self::suggest_context_fix(function_name, expected_context)),
            span,
            help_url: Some("https://leptos.dev/server-client-boundaries".to_string()),
        }
    }

    /// Create invalid feature error
    pub fn invalid_feature(
        feature: &str,
        current_mode: &str,
        span: Option<Span>,
    ) -> Self {
        Self {
            error_type: ValidationErrorType::InvalidFeature,
            message: format!(
                "Feature '{}' is not valid for {} mode",
                feature, current_mode
            ),
            suggestion: Some(format!(
                "Remove '{}' feature or switch to a compatible mode", 
                feature
            )),
            span,
            help_url: Some("https://leptos.dev/modes".to_string()),
        }
    }

    fn suggest_context_fix(function_name: &str, expected_context: &str) -> String {
        match expected_context {
            "server" => format!(
                "Move '{}' to a server function or server component. Use #[server] or create_resource for client data loading.",
                function_name
            ),
            "client" => format!(
                "Move '{}' to client-side code or use create_effect for client-only operations.",
                function_name
            ),
            _ => format!("Use '{}' in {} context only.", function_name, expected_context),
        }
    }

    /// Convert to compile error token stream
    pub fn to_compile_error(&self) -> TokenStream {
        let message = &self.message;
        let suggestion = self.suggestion.as_deref().unwrap_or("");
        let help_url = self.help_url.as_deref().unwrap_or("");

        let full_message = if !suggestion.is_empty() {
            if !help_url.is_empty() {
                format!("{}\n\nSuggestion: {}\nHelp: {}", message, suggestion, help_url)
            } else {
                format!("{}\n\nSuggestion: {}", message, suggestion)
            }
        } else {
            message.clone()
        };

        let span = self.span.unwrap_or_else(Span::call_site);
        syn::Error::new(span, full_message).to_compile_error()
    }
}


/// Validate feature flag combinations at compile time
pub fn validate_features() -> TokenStream {
    let context = ValidationContext::from_env();
    let mut errors = Vec::new();

    // Check for conflicting features
    let conflicting_sets = vec![
        vec!["csr", "ssr"],
        vec!["csr", "hydrate"],
    ];

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

    // Check mode-specific feature validity
    if let Some(mode) = context.current_mode {
        let resolver = ModeResolver::new(mode.clone());
        
        for feature in &context.enabled_features {
            if !resolver.is_feature_valid(feature) {
                errors.push(ValidationError::invalid_feature(
                    feature,
                    &format!("{:?}", mode),
                    None,
                ));
            }
        }
    }

    // Generate compile errors
    let error_tokens: Vec<TokenStream> = errors
        .iter()
        .map(|error| error.to_compile_error())
        .collect();

    quote! {
        #(#error_tokens)*
    }
}

/// Enhanced macro that validates signal usage patterns
pub fn validate_signal_usage(input: TokenStream) -> TokenStream {
    // This would analyze signal usage patterns and suggest optimizations
    // For now, just pass through
    input
}


/// Build-time performance analysis
pub struct PerformanceAnalyzer {
    signal_count: usize,
    effect_count: usize,
    component_count: usize,
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            signal_count: 0,
            effect_count: 0,
            component_count: 0,
        }
    }

    pub fn analyze_performance(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.signal_count > 1000 {
            warnings.push(format!(
                "High signal count ({}). Consider consolidating signals or using stores.",
                self.signal_count
            ));
        }

        if self.effect_count > 500 {
            warnings.push(format!(
                "High effect count ({}). Consider batching effects or using derived signals.",
                self.effect_count
            ));
        }

        if self.component_count > 100 {
            warnings.push(format!(
                "High component count ({}). Consider component memoization.",
                self.component_count
            ));
        }

        warnings
    }
}

/// Integration with leptos_init for automatic validation setup
pub fn setup_validation_in_project(project_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml_path = project_path.join("Cargo.toml");
    let mut cargo_content = std::fs::read_to_string(&cargo_toml_path)?;

    // Add validation dependencies
    if !cargo_content.contains("leptos_compile_validator") {
        cargo_content.push_str(&format!(
            r#"
[dependencies]
leptos_compile_validator = {{ path = "../leptos_compile_validator" }}
leptos_compile_validator_derive = {{ path = "../leptos_compile_validator_derive" }}

[build-dependencies]
leptos_compile_validator = {{ path = "../leptos_compile_validator", features = ["build"] }}
"#
        ));

        std::fs::write(&cargo_toml_path, cargo_content)?;
    }

    // Create build.rs for validation
    let build_rs_content = r#"
use leptos_compile_validator::validate_features;

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTOS_MODE");
    println!("cargo:rerun-if-env-changed=LEPTOS_TARGET");
    
    // Perform compile-time validation
    let validation_result = validate_features();
    if !validation_result.is_empty() {
        panic!("Leptos validation failed");
    }
}
"#;

    std::fs::write(project_path.join("build.rs"), build_rs_content)?;

    Ok(())
}

/// Enhanced validation that includes context-aware checks
pub fn validate_with_context() -> TokenStream {
    let context = ValidationContext::from_env();
    let mut errors = Vec::new();

    // Standard feature validation
    let feature_validation = validate_features();
    
    // Context-aware validation
    if let Some(mode) = context.current_mode {
        let resolver = ModeResolver::new(mode.clone());
        
        // Check for context mismatches
        for feature in &context.enabled_features {
            if !resolver.is_feature_valid(feature) {
                errors.push(ValidationError::invalid_feature(
                    feature,
                    &format!("{:?}", mode),
                    None,
                ));
            }
        }
        
        // Validate mode-specific requirements
        match mode {
            BuildMode::Spa => {
                if context.enabled_features.contains(&"ssr".to_string()) {
                    errors.push(ValidationError::invalid_feature(
                        "ssr",
                        "SPA mode",
                        None,
                    ));
                }
            }
            BuildMode::Fullstack => {
                // Fullstack mode should have both ssr and hydrate
                if !context.enabled_features.contains(&"ssr".to_string()) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::MissingFeature,
                        message: "Fullstack mode requires 'ssr' feature for server builds".to_string(),
                        suggestion: Some("Enable 'ssr' feature or use a different mode".to_string()),
                        span: None,
                        help_url: Some("https://leptos.dev/modes#fullstack".to_string()),
                    });
                }
            }
            _ => {}
        }
    }

    // Generate compile errors
    let error_tokens: Vec<TokenStream> = errors
        .iter()
        .map(|error| error.to_compile_error())
        .collect();

    quote! {
        #feature_validation
        #(#error_tokens)*
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_context() {
        let mut context = ValidationContext::from_env();
        
        context.enabled_features.insert("csr".to_string());
        context.enabled_features.insert("ssr".to_string());
        context.current_mode = Some(BuildMode::Spa);

        // Should detect conflict
        let resolver = ModeResolver::new(BuildMode::Spa);
        let conflicts = resolver.detect_conflicts(&["csr".to_string(), "ssr".to_string()]);
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn test_error_messages() {
        let error = ValidationError::feature_conflict(
            vec!["csr".to_string(), "ssr".to_string()],
            None,
        );

        assert!(error.message.contains("csr, ssr"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_context_validation() {
        let error = ValidationError::wrong_context(
            "database_query",
            "server",
            "client",
            None,
        );

        assert!(error.message.contains("server context"));
        assert!(error.suggestion.as_ref().unwrap().contains("server function"));
    }
}