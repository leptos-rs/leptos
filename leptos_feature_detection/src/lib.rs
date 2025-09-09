//! Automatic feature flag detection and build mode management for Leptos
//! 
//! This crate provides intelligent build mode detection to eliminate the need for manual
//! feature flag configuration. It analyzes project structure and usage patterns to
//! automatically determine the appropriate build configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

pub mod detection;
pub mod mode;
pub mod validation;

/// Supported Leptos build modes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeptosMode {
    /// Client-side rendering only
    CSR,
    /// Server-side rendering with hydration
    SSR,
    /// Hydration mode (client-side hydration of server-rendered content)
    Hydrate,
    /// Full-stack mode (automatic SSR/hydration selection)
    Fullstack,
    /// Static site generation
    Static,
    /// Islands architecture
    Islands,
}

impl LeptosMode {
    /// Get the required Cargo features for this mode
    pub fn required_features(&self) -> Vec<&'static str> {
        match self {
            LeptosMode::CSR => vec!["csr"],
            LeptosMode::SSR => vec!["ssr"],
            LeptosMode::Hydrate => vec!["hydrate"],
            LeptosMode::Fullstack => vec!["ssr", "hydrate"],
            LeptosMode::Static => vec!["ssr"],
            LeptosMode::Islands => vec!["islands"],
        }
    }

    /// Get the bin target features for this mode
    pub fn bin_features(&self) -> Vec<&'static str> {
        match self {
            LeptosMode::CSR => vec![],
            LeptosMode::SSR | LeptosMode::Fullstack | LeptosMode::Static => vec!["ssr"],
            LeptosMode::Hydrate => vec!["hydrate"],
            LeptosMode::Islands => vec!["islands"],
        }
    }

    /// Get the lib target features for this mode
    pub fn lib_features(&self) -> Vec<&'static str> {
        match self {
            LeptosMode::CSR => vec!["csr"],
            LeptosMode::SSR => vec!["ssr"],
            LeptosMode::Hydrate | LeptosMode::Fullstack => vec!["hydrate"],
            LeptosMode::Static => vec!["ssr"],
            LeptosMode::Islands => vec!["islands", "hydrate"],
        }
    }

    /// Check if this mode is compatible with the given features
    pub fn is_compatible_with_features(&self, features: &[String]) -> bool {
        let required = self.required_features();
        let feature_strs: Vec<&str> = features.iter().map(|s| s.as_str()).collect();
        
        // Check for conflicting features
        let conflicts = [
            ("csr", "ssr"),
            ("csr", "hydrate"),
            ("ssr", "hydrate"),
        ];
        
        for (a, b) in conflicts {
            if feature_strs.contains(&a) && feature_strs.contains(&b) && *self != LeptosMode::Fullstack {
                return false;
            }
        }
        
        // Check if all required features are present
        required.iter().all(|req| feature_strs.contains(req))
    }
}

/// Configuration for Leptos mode detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Explicit mode declaration
    pub mode: Option<LeptosMode>,
    /// Target-specific mode configuration
    pub targets: HashMap<String, LeptosMode>,
    /// Force specific features
    pub force_features: Vec<String>,
    /// Disable automatic detection
    pub disable_auto_detection: bool,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            mode: None,
            targets: HashMap::new(),
            force_features: Vec::new(),
            disable_auto_detection: false,
        }
    }
}

/// Results of feature flag analysis
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Detected mode based on project structure
    pub detected_mode: LeptosMode,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Issues found in current configuration
    pub issues: Vec<ConfigIssue>,
    /// Recommended configuration changes
    pub recommendations: Vec<Recommendation>,
    /// Current feature flags in use
    pub current_features: Vec<String>,
}

/// Configuration issues detected
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
}

/// Issue severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Recommended configuration changes
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub action: String,
    pub rationale: String,
    pub file: Option<PathBuf>,
    pub before: Option<String>,
    pub after: String,
}

/// Main analyzer for detecting Leptos project modes
pub struct ModeDetector {
    project_root: PathBuf,
}

impl ModeDetector {
    /// Create a new mode detector for the given project root
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Analyze the project and detect the appropriate mode
    pub fn analyze(&self) -> Result<AnalysisResult, DetectionError> {
        let mut analysis = AnalysisResult {
            detected_mode: LeptosMode::CSR,
            confidence: 0.0,
            issues: Vec::new(),
            recommendations: Vec::new(),
            current_features: Vec::new(),
        };

        // Read current Cargo.toml
        let cargo_toml = self.project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(DetectionError::CargoTomlNotFound);
        }

        // Analyze project structure
        self.analyze_project_structure(&mut analysis)?;
        
        // Analyze Cargo.toml configuration
        self.analyze_cargo_config(&mut analysis)?;
        
        // Analyze source code patterns
        self.analyze_source_code(&mut analysis)?;
        
        // Generate recommendations
        self.generate_recommendations(&mut analysis)?;

        Ok(analysis)
    }

    fn analyze_project_structure(&self, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        let src_dir = self.project_root.join("src");
        let mut has_main_rs = false;
        let mut has_lib_rs = false;
        let mut has_server_code = false;
        let mut has_client_code = false;

        if src_dir.exists() {
            if src_dir.join("main.rs").exists() {
                has_main_rs = true;
            }
            if src_dir.join("lib.rs").exists() {
                has_lib_rs = true;
            }

            // Check for server-specific files
            if src_dir.join("server").exists() || 
               src_dir.join("api").exists() ||
               src_dir.join("handlers").exists() {
                has_server_code = true;
            }

            // Check for client-specific files
            if src_dir.join("client").exists() ||
               src_dir.join("components").exists() ||
               src_dir.join("pages").exists() {
                has_client_code = true;
            }
        }

        // Detect mode based on structure
        let (mode, confidence) = match (has_main_rs, has_lib_rs, has_server_code, has_client_code) {
            (true, true, true, true) => (LeptosMode::Fullstack, 0.9),
            (true, true, true, false) => (LeptosMode::SSR, 0.8),
            (true, true, false, true) => (LeptosMode::Hydrate, 0.8),
            (false, true, false, true) => (LeptosMode::CSR, 0.9),
            (true, false, false, false) => (LeptosMode::CSR, 0.7),
            (true, true, false, false) => (LeptosMode::Fullstack, 0.6),
            _ => (LeptosMode::CSR, 0.5),
        };

        analysis.detected_mode = mode;
        analysis.confidence = confidence;

        Ok(())
    }

    fn analyze_cargo_config(&self, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|_| DetectionError::CargoTomlNotFound)?;

        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| DetectionError::InvalidCargoToml(e.to_string()))?;

        // Extract current features
        if let Some(features) = cargo_toml.get("features") {
            if let Some(default_features) = features.get("default") {
                if let Some(array) = default_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            analysis.current_features.push(feature_str.to_string());
                        }
                    }
                }
            }
        }

        // Check for existing leptos metadata
        if let Some(metadata) = cargo_toml.get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("leptos"))
        {
            // Analyze existing configuration
            if let Some(bin_features) = metadata.get("bin-features") {
                if let Some(array) = bin_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            if feature_str == "ssr" {
                                analysis.detected_mode = LeptosMode::SSR;
                                analysis.confidence = f32::max(analysis.confidence, 0.8);
                            }
                        }
                    }
                }
            }

            if let Some(lib_features) = metadata.get("lib-features") {
                if let Some(array) = lib_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            if feature_str == "hydrate" {
                                if analysis.detected_mode == LeptosMode::SSR {
                                    analysis.detected_mode = LeptosMode::Fullstack;
                                } else {
                                    analysis.detected_mode = LeptosMode::Hydrate;
                                }
                                analysis.confidence = f32::max(analysis.confidence, 0.8);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn analyze_source_code(&self, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        let src_dir = self.project_root.join("src");
        if !src_dir.exists() {
            return Ok(());
        }

        let mut visitor = CodeAnalysisVisitor::new();
        self.visit_rust_files(&src_dir, &mut visitor)?;

        // Update mode based on code analysis
        if visitor.has_server_functions && visitor.has_hydration {
            analysis.detected_mode = LeptosMode::Fullstack;
            analysis.confidence = f32::max(analysis.confidence, 0.9);
        } else if visitor.has_server_functions {
            analysis.detected_mode = LeptosMode::SSR;
            analysis.confidence = f32::max(analysis.confidence, 0.8);
        } else if visitor.has_hydration {
            analysis.detected_mode = LeptosMode::Hydrate;
            analysis.confidence = f32::max(analysis.confidence, 0.8);
        } else if visitor.has_client_only {
            analysis.detected_mode = LeptosMode::CSR;
            analysis.confidence = f32::max(analysis.confidence, 0.8);
        }

        // Check for configuration issues
        if visitor.has_conflicting_cfg_features {
            analysis.issues.push(ConfigIssue {
                severity: Severity::Error,
                message: "Conflicting feature flags detected in conditional compilation".to_string(),
                file: None,
                line: None,
                suggestion: Some("Use automatic mode detection instead of manual feature flags".to_string()),
            });
        }

        Ok(())
    }

    fn visit_rust_files(&self, dir: &Path, visitor: &mut CodeAnalysisVisitor) -> Result<(), DetectionError> {
        for entry in std::fs::read_dir(dir).map_err(|_| DetectionError::IoError)? {
            let entry = entry.map_err(|_| DetectionError::IoError)?;
            let path = entry.path();
            
            if path.is_dir() {
                self.visit_rust_files(&path, visitor)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    let content = std::fs::read_to_string(&path)
                        .map_err(|_| DetectionError::IoError)?;
                    
                    if let Ok(syntax) = syn::parse_file(&content) {
                        visitor.visit_file(&syntax);
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_recommendations(&self, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        let cargo_toml = self.project_root.join("Cargo.toml");
        
        // Recommend mode declaration
        if !analysis.current_features.is_empty() {
            analysis.recommendations.push(Recommendation {
                action: "Add mode declaration".to_string(),
                rationale: "Replace manual feature flags with automatic mode detection".to_string(),
                file: Some(cargo_toml.clone()),
                before: Some("[features]\ndefault = [\"ssr\", \"hydrate\"]".to_string()),
                after: "[package.metadata.leptos]\nmode = \"fullstack\"".to_string(),
            });
        }

        // Recommend removing conflicting features
        if analysis.current_features.contains(&"ssr".to_string()) && 
           analysis.current_features.contains(&"hydrate".to_string()) {
            analysis.recommendations.push(Recommendation {
                action: "Remove conflicting features".to_string(),
                rationale: "Multiple rendering mode features can cause build issues".to_string(),
                file: Some(cargo_toml),
                before: Some("default = [\"ssr\", \"hydrate\"]".to_string()),
                after: "# Features managed automatically by leptos-mode-detect".to_string(),
            });
        }

        Ok(())
    }
}

/// Visitor for analyzing Rust source code patterns
struct CodeAnalysisVisitor {
    has_server_functions: bool,
    has_hydration: bool,
    has_client_only: bool,
    has_conflicting_cfg_features: bool,
}

impl CodeAnalysisVisitor {
    fn new() -> Self {
        Self {
            has_server_functions: false,
            has_hydration: false,
            has_client_only: false,
            has_conflicting_cfg_features: false,
        }
    }
}

impl<'ast> Visit<'ast> for CodeAnalysisVisitor {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if let Ok(meta) = attr.parse_meta() {
            match &meta {
                syn::Meta::List(list) if list.path.is_ident("cfg") => {
                    // Check for feature flag configurations
                    let tokens = list.nested.to_token_stream().to_string();
                    if tokens.contains("feature = \"ssr\"") {
                        self.has_server_functions = true;
                    }
                    if tokens.contains("feature = \"hydrate\"") {
                        self.has_hydration = true;
                    }
                    if tokens.contains("not(feature = \"ssr\")") {
                        self.has_client_only = true;
                    }
                    
                    // Check for conflicting configurations
                    if (tokens.contains("ssr") && tokens.contains("hydrate")) ||
                       (tokens.contains("csr") && tokens.contains("ssr")) {
                        self.has_conflicting_cfg_features = true;
                    }
                }
                syn::Meta::Path(path) if path.is_ident("server") => {
                    self.has_server_functions = true;
                }
                _ => {}
            }
        }
        
        // Continue visiting nested attributes
        syn::visit::visit_attribute(self, attr);
    }

    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        // Check for server function patterns
        for attr in &func.attrs {
            if let Ok(meta) = attr.parse_meta() {
                if let syn::Meta::Path(path) = meta {
                    if path.is_ident("server") {
                        self.has_server_functions = true;
                    }
                }
            }
        }

        // Check function body for hydration patterns
        let func_string = quote::quote!(#func).to_string();
        if func_string.contains("mount_to_body") || func_string.contains("hydrate") {
            self.has_hydration = true;
        }

        syn::visit::visit_item_fn(self, func);
    }
}

/// Errors that can occur during mode detection
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("Cargo.toml not found in project root")]
    CargoTomlNotFound,
    
    #[error("Invalid Cargo.toml format: {0}")]
    InvalidCargoToml(String),
    
    #[error("I/O error during analysis")]
    IoError,
    
    #[error("Failed to parse Rust source file")]
    ParseError,
}