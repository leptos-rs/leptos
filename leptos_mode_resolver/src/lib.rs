//! Smart Mode-Based Feature Resolution for Leptos
//!
//! Eliminates feature flag confusion by automatically resolving features based on build targets
//! and project modes, replacing manual feature flag configuration.

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

/// Build mode that automatically resolves feature flags
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuildMode {
    /// Client-side rendering only (SPA)
    Spa,
    /// Server-side rendering with client hydration
    Fullstack,
    /// Static site generation
    Static,
    /// Server functions only (API)
    Api,
    /// Custom mode with explicit feature configuration
    Custom {
        client_features: Vec<String>,
        server_features: Vec<String>,
    },
}

/// Build target (client or server)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildTarget {
    Client,
    Server,
}

/// Intelligent feature resolver that eliminates manual feature flag confusion
pub struct ModeResolver {
    mode: BuildMode,
}

impl ModeResolver {
    /// Create new resolver with specified mode
    pub fn new(mode: BuildMode) -> Self {
        Self { mode }
    }

    /// Resolve features for specific build target
    pub fn resolve_features(&self, target: BuildTarget) -> Result<Vec<String>, ModeResolverError> {
        match (&self.mode, target) {
            // SPA mode - client side only
            (BuildMode::Spa, BuildTarget::Client) => Ok(vec!["csr".to_string()]),
            (BuildMode::Spa, BuildTarget::Server) => {
                Err(ModeResolverError::InvalidTargetForMode {
                    mode: "SPA".to_string(),
                    target: "server".to_string(),
                    suggestion: "SPA mode only supports client builds. Use 'fullstack' mode for server-side rendering.".to_string(),
                })
            }

            // Fullstack mode - both client and server
            (BuildMode::Fullstack, BuildTarget::Client) => Ok(vec!["hydrate".to_string()]),
            (BuildMode::Fullstack, BuildTarget::Server) => Ok(vec!["ssr".to_string()]),

            // Static mode - pre-rendered content
            (BuildMode::Static, BuildTarget::Client) => Ok(vec!["hydrate".to_string()]),
            (BuildMode::Static, BuildTarget::Server) => Ok(vec!["ssr".to_string()]),

            // API mode - server only
            (BuildMode::Api, BuildTarget::Client) => {
                Err(ModeResolverError::InvalidTargetForMode {
                    mode: "API".to_string(),
                    target: "client".to_string(),
                    suggestion: "API mode only supports server builds. Use 'spa' or 'fullstack' mode for client builds.".to_string(),
                })
            }
            (BuildMode::Api, BuildTarget::Server) => Ok(vec!["ssr".to_string()]),

            // Custom mode - user defined
            (BuildMode::Custom { client_features, server_features: _ }, BuildTarget::Client) => {
                Ok(client_features.clone())
            }
            (BuildMode::Custom { client_features: _, server_features }, BuildTarget::Server) => {
                Ok(server_features.clone())
            }
        }
    }

    /// Validate mode configuration for common issues
    pub fn validate(&self) -> Result<(), ModeResolverError> {
        match &self.mode {
            BuildMode::Custom { client_features, server_features } => {
                self.validate_custom_features(client_features, server_features)
            }
            _ => Ok(()), // Built-in modes are always valid
        }
    }

    /// Get all possible features for this mode
    pub fn all_features(&self) -> HashSet<String> {
        let mut features = HashSet::new();

        match &self.mode {
            BuildMode::Spa => {
                features.insert("csr".to_string());
            }
            BuildMode::Fullstack => {
                features.insert("ssr".to_string());
                features.insert("hydrate".to_string());
            }
            BuildMode::Static => {
                features.insert("ssr".to_string());
                features.insert("hydrate".to_string());
            }
            BuildMode::Api => {
                features.insert("ssr".to_string());
            }
            BuildMode::Custom { client_features, server_features } => {
                features.extend(client_features.iter().cloned());
                features.extend(server_features.iter().cloned());
            }
        }

        features
    }

    /// Check if specific feature is valid for this mode
    pub fn is_feature_valid(&self, feature: &str) -> bool {
        self.all_features().contains(feature)
    }

    /// Get cargo build command for target
    pub fn build_command(&self, target: BuildTarget) -> Result<String, ModeResolverError> {
        let features = self.resolve_features(target)?;
        let features_flag = if features.is_empty() {
            String::new()
        } else {
            format!("--features \"{}\"", features.join(","))
        };

        let command = match target {
            BuildTarget::Client => {
                format!("cargo build --target wasm32-unknown-unknown {}", features_flag)
            }
            BuildTarget::Server => {
                format!("cargo build {}", features_flag)
            }
        };

        Ok(command.trim().to_string())
    }

    /// Detect conflicting feature combinations
    pub fn detect_conflicts(&self, provided_features: &[String]) -> Vec<FeatureConflict> {
        let mut conflicts = Vec::new();
        let provided_set: HashSet<_> = provided_features.iter().collect();

        // Check for mutually exclusive features
        let exclusive_groups = vec![
            vec!["csr", "ssr", "hydrate"], // Rendering modes are exclusive
        ];

        for group in exclusive_groups {
            let present: Vec<_> = group
                .iter()
                .filter(|feature| provided_set.contains(&feature.to_string()))
                .collect();

            if present.len() > 1 {
                conflicts.push(FeatureConflict {
                    conflict_type: ConflictType::MutuallyExclusive,
                    features: present.iter().map(|s| s.to_string()).collect(),
                    suggestion: format!(
                        "Use mode-based configuration instead of manual features. These features are mutually exclusive: {}",
                        present.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }

        // Check for invalid combinations with current mode
        for feature in provided_features {
            if !self.is_feature_valid(feature) {
                conflicts.push(FeatureConflict {
                    conflict_type: ConflictType::InvalidForMode,
                    features: vec![feature.clone()],
                    suggestion: format!(
                        "Feature '{}' is not valid for {:?} mode",
                        feature, self.mode
                    ),
                });
            }
        }

        conflicts
    }

    fn validate_custom_features(
        &self,
        client_features: &[String],
        server_features: &[String],
    ) -> Result<(), ModeResolverError> {
        // Check for invalid feature names
        let valid_leptos_features = vec![
            "csr", "ssr", "hydrate", "tracing", "nightly", "islands", "delegation",
        ];

        for feature in client_features.iter().chain(server_features.iter()) {
            if !valid_leptos_features.contains(&feature.as_str()) {
                return Err(ModeResolverError::InvalidFeature {
                    feature: feature.clone(),
                    valid_features: valid_leptos_features.iter().map(|s| s.to_string()).collect(),
                });
            }
        }

        // Check for logical conflicts
        let client_set: HashSet<_> = client_features.iter().collect();
        let server_set: HashSet<_> = server_features.iter().collect();

        // SSR should not be in client features
        if client_set.contains(&"ssr".to_string()) {
            return Err(ModeResolverError::LogicalConflict {
                issue: "SSR feature cannot be used in client build".to_string(),
                suggestion: "Move 'ssr' to server_features".to_string(),
            });
        }

        // CSR should not be in server features
        if server_set.contains(&"csr".to_string()) {
            return Err(ModeResolverError::LogicalConflict {
                issue: "CSR feature cannot be used in server build".to_string(),
                suggestion: "Move 'csr' to client_features".to_string(),
            });
        }

        Ok(())
    }
}

/// Errors that can occur during mode resolution
#[derive(Debug, thiserror::Error)]
pub enum ModeResolverError {
    #[error("Invalid target '{target}' for mode '{mode}': {suggestion}")]
    InvalidTargetForMode {
        mode: String,
        target: String,
        suggestion: String,
    },

    #[error("Invalid feature '{feature}'. Valid features: {}", valid_features.join(", "))]
    InvalidFeature {
        feature: String,
        valid_features: Vec<String>,
    },

    #[error("Logical conflict: {issue}. {suggestion}")]
    LogicalConflict { issue: String, suggestion: String },
}

/// Feature conflict detection
#[derive(Debug, Clone)]
pub struct FeatureConflict {
    pub conflict_type: ConflictType,
    pub features: Vec<String>,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    MutuallyExclusive,
    InvalidForMode,
    DependencyMissing,
}

/// Build configuration that replaces manual feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub mode: BuildMode,
    pub additional_features: Vec<String>,
    pub environment: Environment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Production,
    Test,
}

impl BuildConfig {
    /// Create development configuration
    pub fn development(mode: BuildMode) -> Self {
        Self {
            mode,
            additional_features: vec!["tracing".to_string()], // Enable tracing in dev
            environment: Environment::Development,
        }
    }

    /// Create production configuration
    pub fn production(mode: BuildMode) -> Self {
        Self {
            mode,
            additional_features: vec![], // Minimal features in prod
            environment: Environment::Production,
        }
    }

    /// Generate complete feature list for target
    pub fn complete_features(&self, target: BuildTarget) -> Result<Vec<String>, ModeResolverError> {
        let resolver = ModeResolver::new(self.mode.clone());
        let mut features = resolver.resolve_features(target)?;
        features.extend(self.additional_features.clone());
        Ok(features)
    }

    /// Generate cargo-leptos configuration
    pub fn leptos_metadata(&self) -> LeptosMetadata {
        let resolver = ModeResolver::new(self.mode.clone());

        let (bin_features, lib_features) = match &self.mode {
            BuildMode::Spa => (
                vec![],
                resolver.resolve_features(BuildTarget::Client).unwrap_or_default(),
            ),
            BuildMode::Fullstack | BuildMode::Static => (
                resolver.resolve_features(BuildTarget::Server).unwrap_or_default(),
                resolver.resolve_features(BuildTarget::Client).unwrap_or_default(),
            ),
            BuildMode::Api => (
                resolver.resolve_features(BuildTarget::Server).unwrap_or_default(),
                vec![],
            ),
            BuildMode::Custom { client_features, server_features } => {
                (server_features.clone(), client_features.clone())
            }
        };

        LeptosMetadata {
            bin_features,
            lib_features,
            environment: self.environment.clone(),
            mode: self.mode.clone(),
        }
    }
}

/// Generated leptos metadata configuration
#[derive(Debug, Clone)]
pub struct LeptosMetadata {
    pub bin_features: Vec<String>,
    pub lib_features: Vec<String>,
    pub environment: Environment,
    pub mode: BuildMode,
}

impl LeptosMetadata {
    /// Convert to TOML format for Cargo.toml
    pub fn to_toml(&self) -> String {
        let mut toml = String::new();

        if !self.bin_features.is_empty() {
            toml.push_str(&format!(
                "bin-features = [{}]\n",
                self.bin_features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            toml.push_str("bin-default-features = false\n");
        }

        if !self.lib_features.is_empty() {
            toml.push_str(&format!(
                "lib-features = [{}]\n",
                self.lib_features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            toml.push_str("lib-default-features = false\n");
        }

        toml.push_str(&format!("env = \"{}\"\n", match self.environment {
            Environment::Development => "DEV",
            Environment::Production => "PROD", 
            Environment::Test => "TEST",
        }));

        toml.push_str(&format!("# Auto-generated from mode: {:?}\n", self.mode));

        toml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spa_mode_resolution() {
        let resolver = ModeResolver::new(BuildMode::Spa);
        
        assert_eq!(
            resolver.resolve_features(BuildTarget::Client).unwrap(),
            vec!["csr"]
        );
        
        assert!(resolver.resolve_features(BuildTarget::Server).is_err());
    }

    #[test]
    fn test_fullstack_mode_resolution() {
        let resolver = ModeResolver::new(BuildMode::Fullstack);
        
        assert_eq!(
            resolver.resolve_features(BuildTarget::Client).unwrap(),
            vec!["hydrate"]
        );
        
        assert_eq!(
            resolver.resolve_features(BuildTarget::Server).unwrap(),
            vec!["ssr"]
        );
    }

    #[test]
    fn test_api_mode_resolution() {
        let resolver = ModeResolver::new(BuildMode::Api);
        
        assert!(resolver.resolve_features(BuildTarget::Client).is_err());
        
        assert_eq!(
            resolver.resolve_features(BuildTarget::Server).unwrap(),
            vec!["ssr"]
        );
    }

    #[test]
    fn test_custom_mode_validation() {
        let resolver = ModeResolver::new(BuildMode::Custom {
            client_features: vec!["csr".to_string()],
            server_features: vec!["ssr".to_string(), "invalid".to_string()],
        });

        assert!(resolver.validate().is_err());
    }

    #[test]
    fn test_conflict_detection() {
        let resolver = ModeResolver::new(BuildMode::Fullstack);
        let conflicts = resolver.detect_conflicts(&["csr".to_string(), "ssr".to_string()]);
        
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].conflict_type, ConflictType::MutuallyExclusive);
    }

    #[test]
    fn test_build_config_generation() {
        let config = BuildConfig::development(BuildMode::Fullstack);
        let client_features = config.complete_features(BuildTarget::Client).unwrap();
        let server_features = config.complete_features(BuildTarget::Server).unwrap();

        assert!(client_features.contains(&"hydrate".to_string()));
        assert!(client_features.contains(&"tracing".to_string()));
        assert!(server_features.contains(&"ssr".to_string()));
    }

    #[test]
    fn test_leptos_metadata_generation() {
        let config = BuildConfig::production(BuildMode::Fullstack);
        let metadata = config.leptos_metadata();
        let toml = metadata.to_toml();

        assert!(toml.contains("bin-features"));
        assert!(toml.contains("lib-features"));
        assert!(toml.contains("env = \"PROD\""));
    }

    #[test]
    fn test_build_command_generation() {
        let resolver = ModeResolver::new(BuildMode::Spa);
        let client_cmd = resolver.build_command(BuildTarget::Client).unwrap();
        
        assert!(client_cmd.contains("wasm32-unknown-unknown"));
        assert!(client_cmd.contains("csr"));
    }
}