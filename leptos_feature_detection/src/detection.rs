//! Smart detection algorithms for Leptos project modes

use crate::{LeptosMode, AnalysisResult, ConfigIssue, Severity, Recommendation, DetectionError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

/// Advanced project analyzer with heuristics-based detection
pub struct SmartDetector {
    project_root: PathBuf,
    confidence_weights: ConfidenceWeights,
}

/// Weights for different detection signals
#[derive(Debug, Clone)]
struct ConfidenceWeights {
    /// File structure analysis weight
    structure_weight: f32,
    /// Cargo.toml configuration weight
    config_weight: f32,
    /// Source code patterns weight
    code_weight: f32,
    /// Dependencies analysis weight
    deps_weight: f32,
}

impl Default for ConfidenceWeights {
    fn default() -> Self {
        Self {
            structure_weight: 0.3,
            config_weight: 0.4,
            code_weight: 0.2,
            deps_weight: 0.1,
        }
    }
}

impl SmartDetector {
    /// Create a new smart detector
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            confidence_weights: ConfidenceWeights::default(),
        }
    }

    /// Perform comprehensive project analysis
    pub fn analyze_comprehensive(&self) -> Result<AnalysisResult, DetectionError> {
        let mut signals = DetectionSignals::new();
        
        // Collect all detection signals
        self.analyze_file_structure(&mut signals)?;
        self.analyze_cargo_configuration(&mut signals)?;
        self.analyze_source_patterns(&mut signals)?;
        self.analyze_dependencies(&mut signals)?;
        
        // Calculate weighted confidence scores
        let mode_scores = self.calculate_mode_scores(&signals);
        
        // Determine the most likely mode
        let (detected_mode, confidence) = mode_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(mode, score)| (mode.clone(), *score))
            .unwrap_or((LeptosMode::CSR, 0.0));

        // Generate analysis result
        let current_features = signals.current_features.clone();
        let mut analysis = AnalysisResult {
            detected_mode,
            confidence,
            issues: Vec::new(),
            recommendations: Vec::new(),
            current_features,
        };

        // Detect configuration issues
        self.detect_issues(&signals, &mut analysis)?;
        
        // Generate recommendations
        self.generate_smart_recommendations(&signals, &mut analysis)?;

        Ok(analysis)
    }

    fn analyze_file_structure(&self, signals: &mut DetectionSignals) -> Result<(), DetectionError> {
        let src_dir = self.project_root.join("src");
        
        // Check for main files
        signals.has_main_rs = src_dir.join("main.rs").exists();
        signals.has_lib_rs = src_dir.join("lib.rs").exists();
        
        // Check for specialized directories
        signals.has_server_dir = src_dir.join("server").exists() || 
                                 src_dir.join("api").exists() ||
                                 src_dir.join("handlers").exists() ||
                                 src_dir.join("routes").exists();
        
        signals.has_client_dir = src_dir.join("client").exists() ||
                                src_dir.join("components").exists() ||
                                src_dir.join("pages").exists() ||
                                src_dir.join("views").exists();
        
        // Check for static assets
        signals.has_static_assets = self.project_root.join("public").exists() ||
                                   self.project_root.join("static").exists() ||
                                   self.project_root.join("assets").exists();
        
        // Check for build configuration
        signals.has_leptos_config = self.project_root.join("Leptos.toml").exists();
        
        // Count Rust files
        signals.rust_file_count = self.count_rust_files(&src_dir)?;

        Ok(())
    }

    fn analyze_cargo_configuration(&self, signals: &mut DetectionSignals) -> Result<(), DetectionError> {
        let cargo_toml = self.project_root.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)
            .map_err(|_| DetectionError::CargoTomlNotFound)?;

        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| DetectionError::InvalidCargoToml(e.to_string()))?;

        // Analyze crate type
        if let Some(lib) = cargo_toml.get("lib") {
            if let Some(crate_type) = lib.get("crate-type") {
                if let Some(types) = crate_type.as_array() {
                    for crate_type in types {
                        if let Some(type_str) = crate_type.as_str() {
                            match type_str {
                                "cdylib" => signals.has_cdylib = true,
                                "rlib" => signals.has_rlib = true,
                                "bin" => signals.has_bin_target = true,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Check for binary targets
        if cargo_toml.get("bin").is_some() || signals.has_main_rs {
            signals.has_bin_target = true;
        }

        // Analyze features
        if let Some(features) = cargo_toml.get("features") {
            if let Some(default_features) = features.get("default") {
                if let Some(array) = default_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            signals.current_features.push(feature_str.to_string());
                            match feature_str {
                                "ssr" => signals.has_ssr_feature = true,
                                "csr" => signals.has_csr_feature = true,
                                "hydrate" => signals.has_hydrate_feature = true,
                                "islands" => signals.has_islands_feature = true,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Check for leptos metadata
        if let Some(leptos_meta) = cargo_toml.get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("leptos"))
        {
            signals.has_cargo_leptos_config = true;
            
            // Analyze bin/lib features
            if let Some(bin_features) = leptos_meta.get("bin-features") {
                if let Some(array) = bin_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            signals.cargo_leptos_bin_features.push(feature_str.to_string());
                        }
                    }
                }
            }
            
            if let Some(lib_features) = leptos_meta.get("lib-features") {
                if let Some(array) = lib_features.as_array() {
                    for feature in array {
                        if let Some(feature_str) = feature.as_str() {
                            signals.cargo_leptos_lib_features.push(feature_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn analyze_source_patterns(&self, signals: &mut DetectionSignals) -> Result<(), DetectionError> {
        let src_dir = self.project_root.join("src");
        let mut visitor = AdvancedCodeVisitor::new();
        
        self.visit_rust_files(&src_dir, &mut visitor)?;
        
        // Transfer results from visitor to signals
        signals.has_server_functions = visitor.server_function_count > 0;
        signals.server_function_count = visitor.server_function_count;
        signals.has_hydration_code = visitor.has_hydration_patterns;
        signals.has_client_components = visitor.component_count > 0;
        signals.component_count = visitor.component_count;
        signals.has_async_rendering = visitor.has_async_patterns;
        signals.has_static_generation = visitor.has_static_patterns;
        signals.cfg_feature_usage = visitor.cfg_features.clone();
        signals.mount_patterns = visitor.mount_patterns;

        Ok(())
    }

    fn analyze_dependencies(&self, signals: &mut DetectionSignals) -> Result<(), DetectionError> {
        let cargo_toml = self.project_root.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)
            .map_err(|_| DetectionError::CargoTomlNotFound)?;

        let cargo_toml: toml::Value = toml::from_str(&content)
            .map_err(|e| DetectionError::InvalidCargoToml(e.to_string()))?;

        // Analyze dependencies for server-specific libraries
        if let Some(deps) = cargo_toml.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                for (dep_name, _) in deps_table {
                    match dep_name.as_str() {
                        "axum" | "actix-web" | "warp" | "tide" => {
                            signals.has_server_deps = true;
                        }
                        "tokio" => {
                            signals.has_async_deps = true;
                        }
                        "sqlx" | "diesel" | "sea-orm" => {
                            signals.has_database_deps = true;
                        }
                        "leptos_axum" | "leptos_actix" => {
                            signals.has_leptos_server_integration = true;
                        }
                        "wasm-bindgen" => {
                            signals.has_wasm_deps = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn calculate_mode_scores(&self, signals: &DetectionSignals) -> HashMap<LeptosMode, f32> {
        let mut scores = HashMap::new();
        
        // Initialize all modes with base score
        for mode in &[
            LeptosMode::CSR,
            LeptosMode::SSR,
            LeptosMode::Hydrate,
            LeptosMode::Fullstack,
            LeptosMode::Static,
            LeptosMode::Islands,
        ] {
            scores.insert(mode.clone(), 0.0);
        }

        // File structure signals
        let structure_score = self.calculate_structure_scores(signals);
        // Config signals  
        let config_score = self.calculate_config_scores(signals);
        // Code pattern signals
        let code_score = self.calculate_code_scores(signals);
        // Dependency signals
        let deps_score = self.calculate_deps_scores(signals);

        // Combine weighted scores
        let mode_keys: Vec<_> = scores.keys().cloned().collect();
        for mode in mode_keys {
            let total_score = 
                structure_score.get(&mode).unwrap_or(&0.0) * self.confidence_weights.structure_weight +
                config_score.get(&mode).unwrap_or(&0.0) * self.confidence_weights.config_weight +
                code_score.get(&mode).unwrap_or(&0.0) * self.confidence_weights.code_weight +
                deps_score.get(&mode).unwrap_or(&0.0) * self.confidence_weights.deps_weight;
            
            scores.insert(mode, total_score);
        }

        scores
    }

    fn calculate_structure_scores(&self, signals: &DetectionSignals) -> HashMap<LeptosMode, f32> {
        let mut scores = HashMap::new();

        // CSR indicators
        let mut csr_score = 0.0;
        if !signals.has_main_rs && signals.has_lib_rs { csr_score += 0.4; }
        if !signals.has_server_dir && signals.has_client_dir { csr_score += 0.3; }
        if signals.has_wasm_deps && !signals.has_server_deps { csr_score += 0.3; }
        scores.insert(LeptosMode::CSR, csr_score);

        // SSR indicators  
        let mut ssr_score = 0.0;
        if signals.has_main_rs && signals.has_lib_rs { ssr_score += 0.4; }
        if signals.has_server_dir { ssr_score += 0.3; }
        if signals.has_server_deps { ssr_score += 0.3; }
        scores.insert(LeptosMode::SSR, ssr_score);

        // Fullstack indicators
        let mut fullstack_score = 0.0;
        if signals.has_main_rs && signals.has_lib_rs { fullstack_score += 0.2; }
        if signals.has_server_dir && signals.has_client_dir { fullstack_score += 0.4; }
        if signals.has_server_deps && signals.has_wasm_deps { fullstack_score += 0.4; }
        scores.insert(LeptosMode::Fullstack, fullstack_score);

        // Static indicators
        let mut static_score = 0.0;
        if signals.has_static_assets { static_score += 0.3; }
        if signals.has_main_rs && !signals.has_server_deps { static_score += 0.2; }
        scores.insert(LeptosMode::Static, static_score);

        // Hydrate indicators
        let mut hydrate_score = 0.0;
        if signals.has_lib_rs && !signals.has_main_rs { hydrate_score += 0.3; }
        if signals.has_hydration_code { hydrate_score += 0.4; }
        scores.insert(LeptosMode::Hydrate, hydrate_score);

        // Islands indicators
        let mut islands_score = 0.0;
        if signals.has_islands_feature { islands_score += 0.6; }
        if signals.component_count > 5 { islands_score += 0.2; }
        scores.insert(LeptosMode::Islands, islands_score);

        scores
    }

    fn calculate_config_scores(&self, signals: &DetectionSignals) -> HashMap<LeptosMode, f32> {
        let mut scores = HashMap::new();

        // Explicit feature flags
        if signals.has_csr_feature {
            scores.insert(LeptosMode::CSR, 0.8);
        }
        if signals.has_ssr_feature {
            scores.insert(LeptosMode::SSR, 0.8);
        }
        if signals.has_hydrate_feature {
            scores.insert(LeptosMode::Hydrate, 0.8);
        }
        if signals.has_islands_feature {
            scores.insert(LeptosMode::Islands, 0.8);
        }

        // cargo-leptos configuration
        if signals.has_cargo_leptos_config {
            let mut fullstack_score = 0.0;
            
            if signals.cargo_leptos_bin_features.contains(&"ssr".to_string()) {
                fullstack_score += 0.4;
            }
            if signals.cargo_leptos_lib_features.contains(&"hydrate".to_string()) {
                fullstack_score += 0.4;
            }
            
            scores.insert(LeptosMode::Fullstack, fullstack_score);
        }

        // Crate type indicators
        if signals.has_cdylib && signals.has_rlib {
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.3;
        } else if signals.has_cdylib {
            *scores.entry(LeptosMode::CSR).or_insert(0.0) += 0.3;
        } else if signals.has_rlib && signals.has_bin_target {
            *scores.entry(LeptosMode::SSR).or_insert(0.0) += 0.3;
        }

        scores
    }

    fn calculate_code_scores(&self, signals: &DetectionSignals) -> HashMap<LeptosMode, f32> {
        let mut scores = HashMap::new();

        // Server functions indicate SSR/fullstack
        if signals.server_function_count > 0 {
            let server_score = (signals.server_function_count as f32 * 0.1).min(0.5);
            *scores.entry(LeptosMode::SSR).or_insert(0.0) += server_score;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += server_score;
        }

        // Hydration patterns
        if signals.has_hydration_code {
            *scores.entry(LeptosMode::Hydrate).or_insert(0.0) += 0.4;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.4;
        }

        // Component patterns
        if signals.component_count > 0 {
            let component_score = (signals.component_count as f32 * 0.05).min(0.3);
            for mode in &[LeptosMode::CSR, LeptosMode::SSR, LeptosMode::Fullstack, LeptosMode::Hydrate] {
                *scores.entry(mode.clone()).or_insert(0.0) += component_score;
            }
        }

        // Mount patterns
        match signals.mount_patterns {
            MountPattern::MountToBody => {
                *scores.entry(LeptosMode::CSR).or_insert(0.0) += 0.3;
            }
            MountPattern::Hydrate => {
                *scores.entry(LeptosMode::Hydrate).or_insert(0.0) += 0.4;
                *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.3;
            }
            MountPattern::Both => {
                *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.5;
            }
            MountPattern::None => {}
        }

        scores
    }

    fn calculate_deps_scores(&self, signals: &DetectionSignals) -> HashMap<LeptosMode, f32> {
        let mut scores = HashMap::new();

        if signals.has_server_deps {
            *scores.entry(LeptosMode::SSR).or_insert(0.0) += 0.4;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.3;
        }

        if signals.has_wasm_deps {
            *scores.entry(LeptosMode::CSR).or_insert(0.0) += 0.3;
            *scores.entry(LeptosMode::Hydrate).or_insert(0.0) += 0.3;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.2;
        }

        if signals.has_database_deps {
            *scores.entry(LeptosMode::SSR).or_insert(0.0) += 0.3;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.3;
        }

        if signals.has_leptos_server_integration {
            *scores.entry(LeptosMode::SSR).or_insert(0.0) += 0.5;
            *scores.entry(LeptosMode::Fullstack).or_insert(0.0) += 0.5;
        }

        scores
    }

    fn count_rust_files(&self, dir: &Path) -> Result<usize, DetectionError> {
        let mut count = 0;
        if dir.exists() {
            for entry in std::fs::read_dir(dir).map_err(|_| DetectionError::IoError)? {
                let entry = entry.map_err(|_| DetectionError::IoError)?;
                let path = entry.path();
                if path.is_dir() {
                    count += self.count_rust_files(&path)?;
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    fn visit_rust_files(&self, dir: &Path, visitor: &mut AdvancedCodeVisitor) -> Result<(), DetectionError> {
        if !dir.exists() {
            return Ok(());
        }

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

    fn detect_issues(&self, signals: &DetectionSignals, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        // Check for conflicting features
        let feature_conflicts = [
            ("csr", "ssr"),
            ("csr", "hydrate"),
        ];

        for (a, b) in feature_conflicts {
            if signals.current_features.contains(&a.to_string()) && 
               signals.current_features.contains(&b.to_string()) {
                analysis.issues.push(ConfigIssue {
                    severity: Severity::Error,
                    message: format!("Conflicting features detected: '{}' and '{}'", a, b),
                    file: Some(self.project_root.join("Cargo.toml")),
                    line: None,
                    suggestion: Some("Remove conflicting features and use mode declaration".to_string()),
                });
            }
        }

        // Check for missing configuration
        if !signals.has_cargo_leptos_config && 
           (signals.has_ssr_feature || signals.has_hydrate_feature) {
            analysis.issues.push(ConfigIssue {
                severity: Severity::Warning,
                message: "Manual feature flags detected without cargo-leptos configuration".to_string(),
                file: Some(self.project_root.join("Cargo.toml")),
                line: None,
                suggestion: Some("Add [package.metadata.leptos] section with mode declaration".to_string()),
            });
        }

        Ok(())
    }

    fn generate_smart_recommendations(&self, signals: &DetectionSignals, analysis: &mut AnalysisResult) -> Result<(), DetectionError> {
        let cargo_toml = self.project_root.join("Cargo.toml");

        // Recommend mode declaration for detected mode
        if !signals.has_cargo_leptos_config || signals.current_features.len() > 0 {
            let mode_str = match analysis.detected_mode {
                LeptosMode::CSR => "csr",
                LeptosMode::SSR => "ssr", 
                LeptosMode::Hydrate => "hydrate",
                LeptosMode::Fullstack => "fullstack",
                LeptosMode::Static => "static",
                LeptosMode::Islands => "islands",
            };

            analysis.recommendations.push(Recommendation {
                action: "Add automatic mode detection".to_string(),
                rationale: format!("Replace manual configuration with automatic {} mode detection", mode_str),
                file: Some(cargo_toml.clone()),
                before: if signals.current_features.is_empty() {
                    None
                } else {
                    Some(format!("[features]\ndefault = {:?}", signals.current_features))
                },
                after: format!("[package.metadata.leptos]\nmode = \"{}\"", mode_str),
            });
        }

        // Recommend removing redundant bin/lib features if using mode
        if signals.has_cargo_leptos_config && 
           (!signals.cargo_leptos_bin_features.is_empty() || !signals.cargo_leptos_lib_features.is_empty()) {
            analysis.recommendations.push(Recommendation {
                action: "Simplify cargo-leptos configuration".to_string(),
                rationale: "Remove redundant bin-features and lib-features when using mode declaration".to_string(),
                file: Some(cargo_toml),
                before: Some("bin-features = [\"ssr\"]\nlib-features = [\"hydrate\"]".to_string()),
                after: "# Features automatically managed by mode declaration".to_string(),
            });
        }

        Ok(())
    }
}

/// Collected detection signals from project analysis
#[derive(Debug, Default)]
struct DetectionSignals {
    // File structure
    has_main_rs: bool,
    has_lib_rs: bool,
    has_server_dir: bool,
    has_client_dir: bool,
    has_static_assets: bool,
    has_leptos_config: bool,
    rust_file_count: usize,

    // Cargo configuration
    has_cdylib: bool,
    has_rlib: bool,
    has_bin_target: bool,
    has_cargo_leptos_config: bool,
    cargo_leptos_bin_features: Vec<String>,
    cargo_leptos_lib_features: Vec<String>,

    // Features
    current_features: Vec<String>,
    has_ssr_feature: bool,
    has_csr_feature: bool,
    has_hydrate_feature: bool,
    has_islands_feature: bool,

    // Code patterns
    has_server_functions: bool,
    server_function_count: usize,
    has_hydration_code: bool,
    has_client_components: bool,
    component_count: usize,
    has_async_rendering: bool,
    has_static_generation: bool,
    cfg_feature_usage: Vec<String>,
    mount_patterns: MountPattern,

    // Dependencies
    has_server_deps: bool,
    has_async_deps: bool,
    has_database_deps: bool,
    has_leptos_server_integration: bool,
    has_wasm_deps: bool,
}

impl DetectionSignals {
    fn new() -> Self {
        Self::default()
    }
}

/// Detected mount patterns in the code
#[derive(Debug, Clone, PartialEq, Default)]
enum MountPattern {
    #[default]
    None,
    MountToBody,
    Hydrate,
    Both,
}

/// Advanced code visitor for pattern detection
struct AdvancedCodeVisitor {
    server_function_count: usize,
    component_count: usize,
    has_hydration_patterns: bool,
    has_async_patterns: bool,
    has_static_patterns: bool,
    cfg_features: Vec<String>,
    mount_patterns: MountPattern,
}

impl AdvancedCodeVisitor {
    fn new() -> Self {
        Self {
            server_function_count: 0,
            component_count: 0,
            has_hydration_patterns: false,
            has_async_patterns: false,
            has_static_patterns: false,
            cfg_features: Vec::new(),
            mount_patterns: MountPattern::None,
        }
    }
}

impl<'ast> Visit<'ast> for AdvancedCodeVisitor {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        // Check for server function attributes
        for attr in &func.attrs {
            if let Ok(meta) = attr.parse_args::<syn::Meta>() {
                match &meta {
                    syn::Meta::Path(path) if path.is_ident("server") => {
                        self.server_function_count += 1;
                    }
                    syn::Meta::List(list) if list.path.is_ident("component") => {
                        self.component_count += 1;
                    }
                    _ => {}
                }
            }
        }

        // Check function content for patterns
        let func_name = &func.sig.ident.to_string();
        
        if func_name.contains("mount") {
            match self.mount_patterns {
                MountPattern::None => self.mount_patterns = MountPattern::MountToBody,
                MountPattern::Hydrate => self.mount_patterns = MountPattern::Both,
                _ => {}
            }
        }
        
        if func_name.contains("hydrate") {
            self.has_hydration_patterns = true;
            match self.mount_patterns {
                MountPattern::None => self.mount_patterns = MountPattern::Hydrate,
                MountPattern::MountToBody => self.mount_patterns = MountPattern::Both,
                _ => {}
            }
        }
        
        if func_name.contains("async") || func_name.contains("await") {
            self.has_async_patterns = true;
        }
        
        if func_name.contains("static") || func_name.contains("generate") {
            self.has_static_patterns = true;
        }

        syn::visit::visit_item_fn(self, func);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if let Ok(meta) = attr.parse_args::<syn::Meta>() {
            if let syn::Meta::List(list) = meta {
                if list.path.is_ident("cfg") {
                    let tokens = list.tokens.to_string();
                    if tokens.contains("feature =") {
                        self.cfg_features.push(tokens);
                    }
                }
            }
        }
        syn::visit::visit_attribute(self, attr);
    }
}