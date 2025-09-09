//! Mode management and configuration system

use crate::{LeptosMode, ModeConfig, DetectionError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Mode resolver that handles configuration and detection
pub struct ModeResolver {
    config: ModeConfig,
}

impl ModeResolver {
    /// Create a new mode resolver with the given configuration
    pub fn new(config: ModeConfig) -> Self {
        Self { config }
    }

    /// Create a mode resolver from Cargo.toml metadata
    pub fn from_cargo_metadata<P: AsRef<Path>>(cargo_toml: P) -> Result<Self, DetectionError> {
        let content = std::fs::read_to_string(cargo_toml)
            .map_err(|_| DetectionError::CargoTomlNotFound)?;

        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| DetectionError::InvalidCargoToml(e.to_string()))?;

        let config = if let Some(leptos_config) = cargo_toml
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("leptos"))
        {
            Self::parse_leptos_config(leptos_config)?
        } else {
            ModeConfig::default()
        };

        Ok(Self::new(config))
    }

    /// Resolve the mode for a given target
    pub fn resolve_mode(&self, target: Option<&str>) -> LeptosMode {
        // Check for explicit target configuration
        if let Some(target) = target {
            if let Some(mode) = self.config.targets.get(target) {
                return mode.clone();
            }
        }

        // Use explicit mode if set
        if let Some(mode) = &self.config.mode {
            return mode.clone();
        }

        // Default to fullstack if no explicit configuration
        LeptosMode::Fullstack
    }

    /// Get the required features for a target
    pub fn get_target_features(&self, target: Option<&str>) -> Vec<String> {
        let mode = self.resolve_mode(target);
        let mut features = mode.required_features()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // Add forced features
        features.extend(self.config.force_features.iter().cloned());

        // Remove duplicates
        features.sort();
        features.dedup();

        features
    }

    /// Get the bin target features
    pub fn get_bin_features(&self, target: Option<&str>) -> Vec<String> {
        let mode = self.resolve_mode(target);
        mode.bin_features()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the lib target features
    pub fn get_lib_features(&self, target: Option<&str>) -> Vec<String> {
        let mode = self.resolve_mode(target);
        mode.lib_features()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if automatic detection is disabled
    pub fn is_auto_detection_disabled(&self) -> bool {
        self.config.disable_auto_detection
    }

    fn parse_leptos_config(config: &toml::Value) -> Result<ModeConfig, DetectionError> {
        let mut mode_config = ModeConfig::default();

        // Parse mode
        if let Some(mode_value) = config.get("mode") {
            if let Some(mode_str) = mode_value.as_str() {
                mode_config.mode = Some(Self::parse_mode(mode_str)?);
            } else if let Some(mode_table) = mode_value.as_table() {
                // Parse target-specific modes
                for (target, mode_val) in mode_table {
                    if let Some(mode_str) = mode_val.as_str() {
                        mode_config.targets.insert(
                            target.clone(),
                            Self::parse_mode(mode_str)?,
                        );
                    }
                }
            }
        }

        // Parse force_features
        if let Some(features) = config.get("force_features") {
            if let Some(features_array) = features.as_array() {
                for feature in features_array {
                    if let Some(feature_str) = feature.as_str() {
                        mode_config.force_features.push(feature_str.to_string());
                    }
                }
            }
        }

        // Parse disable_auto_detection
        if let Some(disable) = config.get("disable_auto_detection") {
            mode_config.disable_auto_detection = disable.as_bool().unwrap_or(false);
        }

        Ok(mode_config)
    }

    fn parse_mode(mode_str: &str) -> Result<LeptosMode, DetectionError> {
        match mode_str.to_lowercase().as_str() {
            "csr" | "client" => Ok(LeptosMode::CSR),
            "ssr" | "server" => Ok(LeptosMode::SSR),
            "hydrate" | "hydration" => Ok(LeptosMode::Hydrate),
            "fullstack" | "full-stack" | "isomorphic" => Ok(LeptosMode::Fullstack),
            "static" | "ssg" => Ok(LeptosMode::Static),
            "islands" => Ok(LeptosMode::Islands),
            _ => Err(DetectionError::InvalidCargoToml(
                format!("Unknown mode: {}", mode_str)
            )),
        }
    }
}

/// Builder for creating mode configurations
pub struct ModeConfigBuilder {
    config: ModeConfig,
}

impl ModeConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: ModeConfig::default(),
        }
    }

    /// Set the default mode
    pub fn mode(mut self, mode: LeptosMode) -> Self {
        self.config.mode = Some(mode);
        self
    }

    /// Add a target-specific mode
    pub fn target_mode<S: Into<String>>(mut self, target: S, mode: LeptosMode) -> Self {
        self.config.targets.insert(target.into(), mode);
        self
    }

    /// Add a forced feature
    pub fn force_feature<S: Into<String>>(mut self, feature: S) -> Self {
        self.config.force_features.push(feature.into());
        self
    }

    /// Disable automatic detection
    pub fn disable_auto_detection(mut self) -> Self {
        self.config.disable_auto_detection = true;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ModeConfig {
        self.config
    }
}

impl Default for ModeConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mode validation utilities
pub struct ModeValidator;

impl ModeValidator {
    /// Validate that a set of features is internally consistent
    pub fn validate_features(features: &[String]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check for conflicting features
        let feature_strs: Vec<&str> = features.iter().map(|s| s.as_str()).collect();
        
        if feature_strs.contains(&"csr") && feature_strs.contains(&"ssr") {
            errors.push("Cannot use both 'csr' and 'ssr' features simultaneously".to_string());
        }
        
        if feature_strs.contains(&"csr") && feature_strs.contains(&"hydrate") {
            errors.push("Cannot use both 'csr' and 'hydrate' features simultaneously".to_string());
        }

        // Note: ssr + hydrate is valid for fullstack mode
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate that a mode configuration makes sense for the project structure
    pub fn validate_mode_for_project(mode: &LeptosMode, project_path: &Path) -> Result<(), Vec<String>> {
        let mut warnings = Vec::new();
        
        let src_dir = project_path.join("src");
        let has_main_rs = src_dir.join("main.rs").exists();
        let has_lib_rs = src_dir.join("lib.rs").exists();
        
        match mode {
            LeptosMode::CSR => {
                if has_main_rs && !has_lib_rs {
                    warnings.push("CSR mode typically uses a lib.rs file for components".to_string());
                }
            }
            LeptosMode::SSR | LeptosMode::Fullstack => {
                if !has_main_rs {
                    warnings.push("SSR mode requires a main.rs file for the server binary".to_string());
                }
                if !has_lib_rs {
                    warnings.push("SSR mode typically uses a lib.rs file for shared components".to_string());
                }
            }
            LeptosMode::Hydrate => {
                if !has_lib_rs {
                    warnings.push("Hydrate mode requires a lib.rs file for client-side hydration".to_string());
                }
            }
            LeptosMode::Static => {
                if !has_main_rs {
                    warnings.push("Static mode requires a main.rs file for static generation".to_string());
                }
            }
            LeptosMode::Islands => {
                if !has_main_rs || !has_lib_rs {
                    warnings.push("Islands mode typically requires both main.rs and lib.rs files".to_string());
                }
            }
        }
        
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_features() {
        assert_eq!(LeptosMode::CSR.required_features(), vec!["csr"]);
        assert_eq!(LeptosMode::SSR.required_features(), vec!["ssr"]);
        assert_eq!(LeptosMode::Hydrate.required_features(), vec!["hydrate"]);
        assert_eq!(LeptosMode::Fullstack.required_features(), vec!["ssr", "hydrate"]);
    }

    #[test]
    fn test_feature_validation() {
        assert!(ModeValidator::validate_features(&["ssr".to_string()]).is_ok());
        assert!(ModeValidator::validate_features(&["ssr".to_string(), "hydrate".to_string()]).is_ok());
        assert!(ModeValidator::validate_features(&["csr".to_string(), "ssr".to_string()]).is_err());
    }

    #[test]
    fn test_mode_compatibility() {
        assert!(LeptosMode::CSR.is_compatible_with_features(&["csr".to_string()]));
        assert!(LeptosMode::Fullstack.is_compatible_with_features(&["ssr".to_string(), "hydrate".to_string()]));
        assert!(!LeptosMode::CSR.is_compatible_with_features(&["ssr".to_string()]));
    }

    #[test]
    fn test_config_builder() {
        let config = ModeConfigBuilder::new()
            .mode(LeptosMode::Fullstack)
            .target_mode("wasm32-unknown-unknown", LeptosMode::CSR)
            .force_feature("tracing")
            .build();

        assert_eq!(config.mode, Some(LeptosMode::Fullstack));
        assert_eq!(config.targets.get("wasm32-unknown-unknown"), Some(&LeptosMode::CSR));
        assert!(config.force_features.contains(&"tracing".to_string()));
    }
}