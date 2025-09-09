//! Build-time validation framework for Leptos configurations

use crate::{LeptosMode, DetectionError, ConfigIssue, Severity};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Comprehensive validation framework for Leptos project configurations
pub struct ValidationFramework {
    project_root: PathBuf,
    validation_rules: Vec<Box<dyn ValidationRule>>,
}

/// Trait for individual validation rules
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult>;
}

/// Context provided to validation rules
pub struct ValidationContext {
    pub project_root: PathBuf,
    pub detected_mode: LeptosMode,
    pub cargo_toml: toml::Value,
    pub current_features: Vec<String>,
    pub source_files: Vec<PathBuf>,
}

/// Result of a validation check
pub struct ValidationResult {
    pub rule_name: String,
    pub issue: ConfigIssue,
}

impl ValidationFramework {
    /// Create a new validation framework
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        let mut framework = Self {
            project_root: project_root.as_ref().to_path_buf(),
            validation_rules: Vec::new(),
        };
        
        // Add default validation rules
        framework.add_default_rules();
        framework
    }

    /// Add a custom validation rule
    pub fn add_rule<R: ValidationRule + 'static>(&mut self, rule: R) {
        self.validation_rules.push(Box::new(rule));
    }

    /// Run all validation rules against the project
    pub fn validate(&self, mode: &LeptosMode) -> Result<Vec<ConfigIssue>, DetectionError> {
        let context = self.build_validation_context(mode)?;
        let mut all_issues = Vec::new();

        for rule in &self.validation_rules {
            let results = rule.validate(&context);
            for result in results {
                all_issues.push(result.issue);
            }
        }

        // Sort issues by severity
        all_issues.sort_by(|a, b| {
            use Severity::*;
            match (&a.severity, &b.severity) {
                (Error, Error) | (Warning, Warning) | (Info, Info) => std::cmp::Ordering::Equal,
                (Error, _) => std::cmp::Ordering::Less,
                (_, Error) => std::cmp::Ordering::Greater,
                (Warning, Info) => std::cmp::Ordering::Less,
                (Info, Warning) => std::cmp::Ordering::Greater,
            }
        });

        Ok(all_issues)
    }

    fn add_default_rules(&mut self) {
        self.add_rule(ConflictingFeaturesRule);
        self.add_rule(MissingDependenciesRule);
        self.add_rule(InvalidCrateTypeRule);
        self.add_rule(ServerFunctionValidationRule);
        self.add_rule(HydrationValidationRule);
        self.add_rule(BuildTargetValidationRule);
        self.add_rule(PerformanceValidationRule);
        self.add_rule(SecurityValidationRule);
    }

    fn build_validation_context(&self, mode: &LeptosMode) -> Result<ValidationContext, DetectionError> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|_| DetectionError::CargoTomlNotFound)?;

        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| DetectionError::InvalidCargoToml(e.to_string()))?;

        // Extract current features
        let mut current_features = Vec::new();
        if let Some(features) = cargo_toml.get("features") {
            if let Some(default_features) = features.get("default") {
                if let Some(array) = default_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            current_features.push(feature_str.to_string());
                        }
                    }
                }
            }
        }

        // Find all source files
        let source_files = self.find_source_files()?;

        Ok(ValidationContext {
            project_root: self.project_root.clone(),
            detected_mode: mode.clone(),
            cargo_toml,
            current_features,
            source_files,
        })
    }

    fn find_source_files(&self) -> Result<Vec<PathBuf>, DetectionError> {
        let mut files = Vec::new();
        let src_dir = self.project_root.join("src");
        
        if src_dir.exists() {
            self.collect_rust_files(&src_dir, &mut files)?;
        }

        Ok(files)
    }

    fn collect_rust_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), DetectionError> {
        for entry in std::fs::read_dir(dir).map_err(|_| DetectionError::IoError)? {
            let entry = entry.map_err(|_| DetectionError::IoError)?;
            let path = entry.path();
            
            if path.is_dir() {
                self.collect_rust_files(&path, files)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path);
                }
            }
        }
        Ok(())
    }
}

/// Rule: Check for conflicting feature flags
struct ConflictingFeaturesRule;

impl ValidationRule for ConflictingFeaturesRule {
    fn name(&self) -> &str {
        "conflicting_features"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let features = &context.current_features;

        let conflicts = [
            ("csr", "ssr", "Client-side and server-side rendering cannot be used together"),
            ("csr", "hydrate", "Client-side rendering and hydration cannot be used together"),
        ];

        for (feat_a, feat_b, message) in conflicts {
            if features.contains(&feat_a.to_string()) && features.contains(&feat_b.to_string()) {
                results.push(ValidationResult {
                    rule_name: self.name().to_string(),
                    issue: ConfigIssue {
                        severity: Severity::Error,
                        message: format!("Conflicting features: {}", message),
                        file: Some(context.project_root.join("Cargo.toml")),
                        line: None,
                        suggestion: Some("Remove conflicting features or use fullstack mode".to_string()),
                    },
                });
            }
        }

        results
    }
}

/// Rule: Check for missing required dependencies
struct MissingDependenciesRule;

impl ValidationRule for MissingDependenciesRule {
    fn name(&self) -> &str {
        "missing_dependencies"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        
        let required_deps = match context.detected_mode {
            LeptosMode::SSR | LeptosMode::Fullstack => {
                vec![
                    ("tokio", "Required for async server runtime"),
                    ("leptos", "Core Leptos framework"),
                ]
            }
            LeptosMode::CSR | LeptosMode::Hydrate => {
                vec![
                    ("leptos", "Core Leptos framework"),
                    ("wasm-bindgen", "Required for WebAssembly compilation"),
                ]
            }
            LeptosMode::Static => {
                vec![
                    ("leptos", "Core Leptos framework"),
                    ("tokio", "Required for static generation"),
                ]
            }
            LeptosMode::Islands => {
                vec![
                    ("leptos", "Core Leptos framework"),
                    ("wasm-bindgen", "Required for island hydration"),
                ]
            }
        };

        if let Some(dependencies) = context.cargo_toml.get("dependencies") {
            if let Some(deps_table) = dependencies.as_table() {
                for (required_dep, reason) in required_deps {
                    if !deps_table.contains_key(required_dep) {
                        results.push(ValidationResult {
                            rule_name: self.name().to_string(),
                            issue: ConfigIssue {
                                severity: Severity::Error,
                                message: format!("Missing required dependency: {}", required_dep),
                                file: Some(context.project_root.join("Cargo.toml")),
                                line: None,
                                suggestion: Some(format!("Add {} to dependencies. {}", required_dep, reason)),
                            },
                        });
                    }
                }
            }
        }

        results
    }
}

/// Rule: Validate crate type configuration
struct InvalidCrateTypeRule;

impl ValidationRule for InvalidCrateTypeRule {
    fn name(&self) -> &str {
        "invalid_crate_type"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        let required_types = match context.detected_mode {
            LeptosMode::CSR => vec!["cdylib"],
            LeptosMode::SSR => vec!["rlib"],
            LeptosMode::Fullstack => vec!["cdylib", "rlib"],
            LeptosMode::Hydrate => vec!["cdylib"],
            LeptosMode::Static => vec!["rlib"],
            LeptosMode::Islands => vec!["cdylib", "rlib"],
        };

        if let Some(lib) = context.cargo_toml.get("lib") {
            if let Some(crate_type) = lib.get("crate-type") {
                if let Some(types) = crate_type.as_array() {
                    let current_types: Vec<&str> = types
                        .iter()
                        .filter_map(|t| t.as_str())
                        .collect();

                    for required_type in &required_types {
                        if !current_types.contains(required_type) {
                            results.push(ValidationResult {
                                rule_name: self.name().to_string(),
                                issue: ConfigIssue {
                                    severity: Severity::Warning,
                                    message: format!("Missing crate type '{}' for {} mode", required_type, 
                                                    mode_display_name(&context.detected_mode)),
                                    file: Some(context.project_root.join("Cargo.toml")),
                                    line: None,
                                    suggestion: Some(format!("Add '{}' to crate-type array", required_type)),
                                },
                            });
                        }
                    }
                }
            } else {
                results.push(ValidationResult {
                    rule_name: self.name().to_string(),
                    issue: ConfigIssue {
                        severity: Severity::Warning,
                        message: "No crate-type specified in [lib] section".to_string(),
                        file: Some(context.project_root.join("Cargo.toml")),
                        line: None,
                        suggestion: Some(format!("Add crate-type = {:?} to [lib] section", required_types)),
                    },
                });
            }
        } else if matches!(context.detected_mode, LeptosMode::CSR | LeptosMode::Fullstack | LeptosMode::Hydrate) {
            results.push(ValidationResult {
                rule_name: self.name().to_string(),
                issue: ConfigIssue {
                    severity: Severity::Error,
                    message: "Missing [lib] section required for client-side compilation".to_string(),
                    file: Some(context.project_root.join("Cargo.toml")),
                    line: None,
                    suggestion: Some("[lib]\ncrate-type = [\"cdylib\", \"rlib\"]".to_string()),
                },
            });
        }

        results
    }
}

/// Rule: Validate server function usage
struct ServerFunctionValidationRule;

impl ValidationRule for ServerFunctionValidationRule {
    fn name(&self) -> &str {
        "server_function_validation"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Check if server functions are used in non-server modes
        if matches!(context.detected_mode, LeptosMode::CSR) {
            for file_path in &context.source_files {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    if content.contains("#[server]") || content.contains("#[server(") {
                        results.push(ValidationResult {
                            rule_name: self.name().to_string(),
                            issue: ConfigIssue {
                                severity: Severity::Error,
                                message: "Server functions cannot be used in CSR mode".to_string(),
                                file: Some(file_path.clone()),
                                line: None,
                                suggestion: Some("Use SSR or fullstack mode for server functions".to_string()),
                            },
                        });
                    }
                }
            }
        }

        results
    }
}

/// Rule: Validate hydration setup
struct HydrationValidationRule;

impl ValidationRule for HydrationValidationRule {
    fn name(&self) -> &str {
        "hydration_validation"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        if matches!(context.detected_mode, LeptosMode::Hydrate | LeptosMode::Fullstack) {
            let mut has_hydrate_call = false;

            for file_path in &context.source_files {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    if content.contains("hydrate") {
                        has_hydrate_call = true;
                        break;
                    }
                }
            }

            if !has_hydrate_call {
                results.push(ValidationResult {
                    rule_name: self.name().to_string(),
                    issue: ConfigIssue {
                        severity: Severity::Warning,
                        message: "Hydration mode detected but no hydrate() call found".to_string(),
                        file: None,
                        line: None,
                        suggestion: Some("Add hydrate() call to your main function".to_string()),
                    },
                });
            }
        }

        results
    }
}

/// Rule: Validate build targets configuration
struct BuildTargetValidationRule;

impl ValidationRule for BuildTargetValidationRule {
    fn name(&self) -> &str {
        "build_target_validation"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Check for appropriate binary targets
        match context.detected_mode {
            LeptosMode::SSR | LeptosMode::Fullstack | LeptosMode::Static => {
                let main_rs_exists = context.project_root.join("src/main.rs").exists();
                let has_bin_section = context.cargo_toml.get("bin").is_some();
                
                if !main_rs_exists && !has_bin_section {
                    results.push(ValidationResult {
                        rule_name: self.name().to_string(),
                        issue: ConfigIssue {
                            severity: Severity::Error,
                            message: "Server mode requires a binary target (main.rs or [[bin]] section)".to_string(),
                            file: Some(context.project_root.join("Cargo.toml")),
                            line: None,
                            suggestion: Some("Create src/main.rs or add [[bin]] section".to_string()),
                        },
                    });
                }
            }
            _ => {}
        }

        results
    }
}

/// Rule: Performance-related validations
struct PerformanceValidationRule;

impl ValidationRule for PerformanceValidationRule {
    fn name(&self) -> &str {
        "performance_validation"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Check for optimization settings in release profile
        if let Some(profiles) = context.cargo_toml.get("profile") {
            if let Some(release) = profiles.get("release") {
                if let Some(release_table) = release.as_table() {
                    if !release_table.contains_key("opt-level") {
                        results.push(ValidationResult {
                            rule_name: self.name().to_string(),
                            issue: ConfigIssue {
                                severity: Severity::Info,
                                message: "Consider adding opt-level for better performance".to_string(),
                                file: Some(context.project_root.join("Cargo.toml")),
                                line: None,
                                suggestion: Some("[profile.release]\nopt-level = 'z'  # or 's' for size optimization".to_string()),
                            },
                        });
                    }
                    
                    if !release_table.contains_key("lto") {
                        results.push(ValidationResult {
                            rule_name: self.name().to_string(),
                            issue: ConfigIssue {
                                severity: Severity::Info,
                                message: "Consider enabling LTO for smaller bundle size".to_string(),
                                file: Some(context.project_root.join("Cargo.toml")),
                                line: None,
                                suggestion: Some("[profile.release]\nlto = true".to_string()),
                            },
                        });
                    }
                }
            }
        }

        results
    }
}

/// Rule: Security-related validations
struct SecurityValidationRule;

impl ValidationRule for SecurityValidationRule {
    fn name(&self) -> &str {
        "security_validation"
    }

    fn validate(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Check for potentially unsafe dependencies in client builds
        if matches!(context.detected_mode, LeptosMode::CSR | LeptosMode::Hydrate | LeptosMode::Fullstack) {
            if let Some(dependencies) = context.cargo_toml.get("dependencies") {
                if let Some(deps_table) = dependencies.as_table() {
                    let server_only_deps = ["sqlx", "diesel", "sea-orm", "tokio-postgres"];
                    
                    for (dep_name, dep_value) in deps_table {
                        if server_only_deps.contains(&dep_name.as_str()) {
                            // Check if dependency is properly feature-gated
                            let is_optional = dep_value.get("optional")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            
                            if !is_optional {
                                results.push(ValidationResult {
                                    rule_name: self.name().to_string(),
                                    issue: ConfigIssue {
                                        severity: Severity::Warning,
                                        message: format!("Server-only dependency '{}' may be included in client builds", dep_name),
                                        file: Some(context.project_root.join("Cargo.toml")),
                                        line: None,
                                        suggestion: Some(format!("Make '{}' optional or feature-gated", dep_name)),
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }

        results
    }
}

fn mode_display_name(mode: &LeptosMode) -> &str {
    match mode {
        LeptosMode::CSR => "CSR",
        LeptosMode::SSR => "SSR", 
        LeptosMode::Hydrate => "hydration",
        LeptosMode::Fullstack => "fullstack",
        LeptosMode::Static => "static",
        LeptosMode::Islands => "islands",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_conflicting_features_rule() {
        let rule = ConflictingFeaturesRule;
        let temp_dir = TempDir::new().unwrap();
        
        let context = ValidationContext {
            project_root: temp_dir.path().to_path_buf(),
            detected_mode: LeptosMode::CSR,
            cargo_toml: toml::Value::Table(toml::map::Map::new()),
            current_features: vec!["csr".to_string(), "ssr".to_string()],
            source_files: Vec::new(),
        };

        let results = rule.validate(&context);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue.severity, Severity::Error);
    }

    #[test]
    fn test_validation_framework() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a minimal Cargo.toml
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[features]
default = ["csr", "ssr"]
            "#
        ).unwrap();
        
        let framework = ValidationFramework::new(temp_dir.path());
        let issues = framework.validate(&LeptosMode::CSR).unwrap();
        
        // Should detect conflicting features
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| matches!(issue.severity, Severity::Error)));
    }
}