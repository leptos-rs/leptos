//! Fast Development Mode
//!
//! Optimized development builds that prioritize speed over optimization:
//! - Reduced optimization levels for faster compilation
//! - Parallel compilation where possible
//! - Smart caching strategies
//! - Development-specific feature flags

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Fast development mode configuration and execution
pub struct FastDevMode {
    project_path: PathBuf,
    config: FastDevConfig,
    build_cache: BuildCache,
    last_build_time: Option<Instant>,
}

/// Configuration for fast development builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastDevConfig {
    /// Optimization level (0 = no optimization, fastest compile)
    pub opt_level: u8,
    /// Enable debug info for better error messages
    pub debug_info: bool,
    /// Number of parallel compilation jobs
    pub jobs: Option<usize>,
    /// Enable incremental compilation
    pub incremental: bool,
    /// Skip tests during development builds
    pub skip_tests: bool,
    /// Enable development-specific features
    pub dev_features: Vec<String>,
    /// Custom cargo flags for development
    pub cargo_flags: Vec<String>,
}

/// Cache for build artifacts and metadata
#[derive(Debug, Default)]
struct BuildCache {
    /// Cache of successful build configurations
    successful_configs: HashMap<String, BuildMetadata>,
    /// Cache of dependency resolution results
    dependency_cache: HashMap<String, DependencyInfo>,
    /// Last successful build hash
    last_build_hash: Option<String>,
}

/// Metadata about a successful build
#[derive(Debug, Clone)]
struct BuildMetadata {
    build_time: Duration,
    modules_compiled: usize,
    artifacts_generated: Vec<PathBuf>,
    timestamp: Instant,
}

/// Dependency information for caching
#[derive(Debug, Clone)]
struct DependencyInfo {
    resolved_deps: Vec<String>,
    dep_hash: String,
    timestamp: Instant,
}

/// Result of a fast development build
#[derive(Debug, Clone)]
pub struct FastDevBuildResult {
    pub success: bool,
    pub build_time: Duration,
    pub modules_compiled: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub artifacts: Vec<PathBuf>,
    pub cache_hit: bool,
}

/// Errors that can occur in fast development mode
#[derive(Debug, thiserror::Error)]
pub enum FastDevError {
    #[error("Project path does not exist: {path}")]
    ProjectNotFound { path: PathBuf },
    
    #[error("Cargo configuration error: {reason}")]
    CargoConfig { reason: String },
    
    #[error("Build execution failed: {reason}")]
    BuildExecution { reason: String },
    
    #[error("Build failed: {reason}")]
    BuildFailed { reason: String },
    
    #[error("Cache corruption: {reason}")]
    CacheCorruption { reason: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
}

impl Default for FastDevConfig {
    fn default() -> Self {
        Self {
            opt_level: 0, // No optimization for fastest compilation
            debug_info: true, // Keep debug info for better error messages
            jobs: None, // Use all available cores
            incremental: true, // Enable incremental compilation
            skip_tests: true, // Skip tests in development
            dev_features: vec!["dev".to_string()],
            cargo_flags: vec!["--dev".to_string()],
        }
    }
}

impl FastDevMode {
    /// Create a new fast development mode instance
    pub fn new<P: AsRef<Path>>(project_path: P) -> Result<Self, FastDevError> {
        let project_path = project_path.as_ref().to_path_buf();
        
        if !project_path.exists() {
            return Err(FastDevError::ProjectNotFound { path: project_path });
        }
        
        // Validate that this is a Rust project
        if !project_path.join("Cargo.toml").exists() {
            return Err(FastDevError::CargoConfig {
                reason: "No Cargo.toml found in project directory".to_string(),
            });
        }
        
        let config = Self::detect_optimal_config(&project_path)?;
        
        Ok(Self {
            project_path,
            config,
            build_cache: BuildCache::default(),
            last_build_time: None,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config<P: AsRef<Path>>(
        project_path: P, 
        config: FastDevConfig
    ) -> Result<Self, FastDevError> {
        let project_path = project_path.as_ref().to_path_buf();
        
        if !project_path.exists() {
            return Err(FastDevError::ProjectNotFound { path: project_path });
        }
        
        Ok(Self {
            project_path,
            config,
            build_cache: BuildCache::default(),
            last_build_time: None,
        })
    }
    
    /// Perform a full project build with fast development optimizations
    pub fn build_project(&mut self) -> Result<FastDevBuildResult, FastDevError> {
        let start_time = Instant::now();
        
        // Check if we can use cached build
        if let Some(cached_result) = self.check_build_cache() {
            return Ok(cached_result);
        }
        
        // Prepare build environment
        self.prepare_build_environment()?;
        
        // Execute the build
        let build_output = self.execute_fast_build()?;
        
        // Process build results
        let result = self.process_build_output(build_output, start_time)?;
        
        // Update cache
        self.update_build_cache()?;
        
        self.last_build_time = Some(start_time);
        Ok(result)
    }
    
    /// Perform an incremental build (faster than full build)
    pub fn incremental_build(&mut self) -> Result<FastDevBuildResult, FastDevError> {
        let start_time = Instant::now();
        
        // For incremental builds, we can be more aggressive with caching
        if let Some(cached_result) = self.check_incremental_cache() {
            return Ok(cached_result);
        }
        
        // Use cargo check for fastest incremental builds
        let build_output = self.execute_incremental_build()?;
        
        let result = self.process_build_output(build_output, start_time)?;
        
        // Update incremental cache
        self.update_incremental_cache(&result);
        
        self.last_build_time = Some(start_time);
        Ok(result)
    }
    
    /// Get current configuration
    pub fn config(&self) -> &FastDevConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: FastDevConfig) {
        self.config = config;
        // Clear cache when configuration changes
        self.build_cache = BuildCache::default();
    }
    
    /// Get build performance metrics
    pub fn get_build_metrics(&self) -> BuildMetrics {
        BuildMetrics {
            last_build_time: self.last_build_time,
            cache_size: self.build_cache.successful_configs.len(),
            config: self.config.clone(),
        }
    }
    
    /// Detect optimal configuration for the project
    fn detect_optimal_config(project_path: &Path) -> Result<FastDevConfig, FastDevError> {
        let mut config = FastDevConfig::default();
        
        // Detect project size and complexity
        let project_size = Self::analyze_project_size(project_path)?;
        
        // Adjust configuration based on project characteristics
        match project_size {
            ProjectSize::Small => {
                // Small projects can use more aggressive optimizations
                config.opt_level = 1;
                config.jobs = Some(2);
            }
            ProjectSize::Medium => {
                // Medium projects balance speed and optimization
                config.opt_level = 0;
                config.jobs = Some(4);
            }
            ProjectSize::Large => {
                // Large projects prioritize compilation speed
                config.opt_level = 0;
                config.jobs = None; // Use all available cores
            }
        }
        
        // Check for specific project features that affect build strategy
        if Self::has_workspace(project_path)? {
            config.cargo_flags.push("--workspace".to_string());
        }
        
        if Self::has_wasm_target(project_path)? {
            config.dev_features.push("wasm".to_string());
        }
        
        Ok(config)
    }
    
    /// Analyze project size and complexity
    fn analyze_project_size(project_path: &Path) -> Result<ProjectSize, FastDevError> {
        let src_dir = project_path.join("src");
        if !src_dir.exists() {
            return Ok(ProjectSize::Small);
        }
        
        let mut file_count = 0;
        for entry in std::fs::read_dir(&src_dir)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "rs") {
                file_count += 1;
            }
        }
        
        Ok(match file_count {
            0..=10 => ProjectSize::Small,
            11..=50 => ProjectSize::Medium,
            _ => ProjectSize::Large,
        })
    }
    
    /// Check if project has workspace configuration
    fn has_workspace(project_path: &Path) -> Result<bool, FastDevError> {
        let cargo_toml = project_path.join("Cargo.toml");
        let content = std::fs::read_to_string(cargo_toml)?;
        Ok(content.contains("[workspace]"))
    }
    
    /// Check if project has WASM target
    fn has_wasm_target(project_path: &Path) -> Result<bool, FastDevError> {
        let cargo_toml = project_path.join("Cargo.toml");
        let content = std::fs::read_to_string(cargo_toml)?;
        Ok(content.contains("wasm32") || content.contains("leptos"))
    }
    
    /// Prepare build environment for optimal performance
    fn prepare_build_environment(&self) -> Result<(), FastDevError> {
        // Set environment variables for faster builds
        std::env::set_var("CARGO_INCREMENTAL", "1");
        std::env::set_var("CARGO_PROFILE_DEV_DEBUG", "1");
        std::env::set_var("CARGO_PROFILE_DEV_OPT_LEVEL", self.config.opt_level.to_string());
        
        if let Some(jobs) = self.config.jobs {
            std::env::set_var("CARGO_BUILD_JOBS", jobs.to_string());
        }
        
        Ok(())
    }
    
    /// Execute fast development build
    fn execute_fast_build(&self) -> Result<Output, FastDevError> {
        let mut cmd = Command::new("cargo");
        
        // Use build command with development optimizations
        cmd.args(["build"]);
        
        // Add configuration flags
        for flag in &self.config.cargo_flags {
            cmd.arg(flag);
        }
        
        // Add feature flags
        if !self.config.dev_features.is_empty() {
            cmd.arg("--features");
            cmd.arg(self.config.dev_features.join(","));
        }
        
        // Skip tests for faster builds
        if self.config.skip_tests {
            cmd.arg("--lib"); // Only build library, skip tests
        }
        
        cmd.current_dir(&self.project_path);
        
        let output = cmd.output()
            .map_err(|e| FastDevError::BuildExecution {
                reason: format!("Failed to execute cargo build: {}", e),
            })?;
        
        Ok(output)
    }
    
    /// Execute incremental build (cargo check)
    fn execute_incremental_build(&self) -> Result<Output, FastDevError> {
        let mut cmd = Command::new("cargo");
        
        // Use check for fastest incremental builds
        cmd.args(["check"]);
        
        // Add configuration flags
        for flag in &self.config.cargo_flags {
            cmd.arg(flag);
        }
        
        // Add feature flags
        if !self.config.dev_features.is_empty() {
            cmd.arg("--features");
            cmd.arg(self.config.dev_features.join(","));
        }
        
        cmd.current_dir(&self.project_path);
        
        let output = cmd.output()
            .map_err(|e| FastDevError::BuildExecution {
                reason: format!("Failed to execute cargo check: {}", e),
            })?;
        
        Ok(output)
    }
    
    /// Process build output and create result
    fn process_build_output(
        &self,
        output: Output,
        start_time: Instant,
    ) -> Result<FastDevBuildResult, FastDevError> {
        let success = output.status.success();
        let build_time = start_time.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Parse warnings and errors
        let warnings = self.extract_warnings(&stderr);
        let errors = self.extract_errors(&stderr);
        
        // Count compiled modules (rough estimate from output)
        let modules_compiled = self.count_compiled_modules(&stdout, &stderr);
        
        // Find generated artifacts
        let artifacts = self.find_build_artifacts()?;
        
        Ok(FastDevBuildResult {
            success,
            build_time,
            modules_compiled,
            warnings,
            errors,
            artifacts,
            cache_hit: false,
        })
    }
    
    /// Extract warning messages from build output
    fn extract_warnings(&self, output: &str) -> Vec<String> {
        output
            .lines()
            .filter(|line| line.contains("warning:"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    /// Extract error messages from build output
    fn extract_errors(&self, output: &str) -> Vec<String> {
        output
            .lines()
            .filter(|line| line.contains("error:"))
            .map(|line| line.trim().to_string())
            .collect()
    }
    
    /// Count compiled modules from build output
    fn count_compiled_modules(&self, stdout: &str, stderr: &str) -> usize {
        let combined = format!("{}\n{}", stdout, stderr);
        
        // Count "Compiling" lines as a rough estimate
        combined
            .lines()
            .filter(|line| line.contains("Compiling"))
            .count()
    }
    
    /// Find build artifacts in target directory
    fn find_build_artifacts(&self) -> Result<Vec<PathBuf>, FastDevError> {
        let target_dir = self.project_path.join("target");
        let mut artifacts = Vec::new();
        
        if target_dir.exists() {
            for entry in walkdir::WalkDir::new(&target_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if matches!(ext.to_str(), Some("rlib") | Some("so") | Some("dylib") | Some("dll")) {
                        artifacts.push(path.to_path_buf());
                    }
                }
            }
        }
        
        Ok(artifacts)
    }
    
    /// Check if we can use cached build result
    fn check_build_cache(&self) -> Option<FastDevBuildResult> {
        // Simple cache check - in practice would be more sophisticated
        None
    }
    
    /// Check if we can use cached incremental build result
    fn check_incremental_cache(&self) -> Option<FastDevBuildResult> {
        // Simple cache check - in practice would be more sophisticated
        None
    }
    
    
    /// Update incremental cache with successful result
    fn update_incremental_cache(&mut self, _result: &FastDevBuildResult) {
        // Update cache - implementation would be more sophisticated
    }
}

/// Project size classification for optimization
#[derive(Debug, Clone, Copy)]
enum ProjectSize {
    Small,
    Medium,
    Large,
}

/// Build performance metrics
#[derive(Debug, Clone)]
pub struct BuildMetrics {
    pub last_build_time: Option<Instant>,
    pub cache_size: usize,
    pub config: FastDevConfig,
}

impl FastDevMode {
    /// Setup fast development configuration
    pub fn setup_fast_config(&self) -> Result<(), FastDevError> {
        println!("⚙️  Setting up fast development configuration...");
        
        // Note: cargo-leptos build doesn't support custom profiles
        // We'll rely on incremental compilation and other optimizations
        
        // Setup incremental compilation
        self.setup_incremental_compilation()?;
        
        // Configure development features
        self.configure_dev_features()?;
        
        println!("✅ Fast development configuration ready");
        Ok(())
    }

    /// Perform a fast build
    pub fn build_fast(&mut self) -> Result<(), FastDevError> {
        let start = std::time::Instant::now();
        
        println!("🔨 Starting fast development build...");
        
        // Check if we can use cached build
        if self.can_use_cached_build()? {
            println!("📦 Using cached build (no changes detected)");
            return Ok(());
        }
        
        // Perform incremental build
        self.perform_incremental_build()?;
        
        // Update build cache
        self.update_build_cache()?;
        
        let duration = start.elapsed();
        self.last_build_time = Some(start);
        
        println!("✅ Fast build completed in {:.2}s", duration.as_secs_f64());
        Ok(())
    }


    /// Setup incremental compilation
    fn setup_incremental_compilation(&self) -> Result<(), FastDevError> {
        // Create .cargo/config.toml if it doesn't exist
        let cargo_config_dir = self.project_path.join(".cargo");
        if !cargo_config_dir.exists() {
            std::fs::create_dir_all(&cargo_config_dir)?;
        }
        
        let config_path = cargo_config_dir.join("config.toml");
        let config_content = r#"
[build]
incremental = true

[target.'cfg(not(target_arch = "wasm32"))']
rustflags = ["-C", "target-cpu=native"]

[target.'cfg(target_arch = "wasm32")']
rustflags = ["-C", "target-feature=+bulk-memory"]
"#;
        
        std::fs::write(&config_path, config_content)?;
        Ok(())
    }

    /// Configure development features
    fn configure_dev_features(&self) -> Result<(), FastDevError> {
        // This would configure development-specific features
        // For now, we'll just ensure the configuration is ready
        Ok(())
    }

    /// Check if we can use a cached build
    fn can_use_cached_build(&self) -> Result<bool, FastDevError> {
        // Simple check: if no source files changed since last build
        if let Some(last_build) = self.last_build_time {
            let src_dir = self.project_path.join("src");
            if src_dir.exists() {
                let mut latest_mtime = std::time::SystemTime::UNIX_EPOCH;
                
                for entry in walkdir::WalkDir::new(&src_dir) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        let metadata = entry.metadata()?;
                        if let Ok(mtime) = metadata.modified() {
                            if mtime > latest_mtime {
                                latest_mtime = mtime;
                            }
                        }
                    }
                }
                
                if let Ok(duration) = latest_mtime.duration_since(std::time::UNIX_EPOCH) {
                    // Convert Instant to SystemTime for comparison
                    let build_time = std::time::SystemTime::now() - last_build.elapsed();
                    if let Ok(build_duration) = build_time.duration_since(std::time::UNIX_EPOCH) {
                        if duration < build_duration {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Perform incremental build
    fn perform_incremental_build(&self) -> Result<(), FastDevError> {
        let mut cmd = Command::new("cargo");
        cmd.arg("leptos")
            .arg("build")
            .current_dir(&self.project_path);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FastDevError::BuildFailed {
                reason: format!("Fast build failed: {}", stderr),
            });
        }
        
        Ok(())
    }

    /// Update build cache
    fn update_build_cache(&mut self) -> Result<(), FastDevError> {
        // Update cache with current build metadata
        let build_hash = self.calculate_build_hash()?;
        self.build_cache.last_build_hash = Some(build_hash);
        Ok(())
    }

    /// Calculate build hash for caching
    fn calculate_build_hash(&self) -> Result<String, FastDevError> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash source files
        let src_dir = self.project_path.join("src");
        if src_dir.exists() {
            for entry in walkdir::WalkDir::new(&src_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let content = std::fs::read(entry.path())?;
                    content.hash(&mut hasher);
                }
            }
        }
        
        // Hash Cargo.toml
        let cargo_toml = self.project_path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read(&cargo_toml)?;
            content.hash(&mut hasher);
        }
        
        Ok(format!("{:x}", hasher.finish()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_fast_dev_config_default() {
        let config = FastDevConfig::default();
        assert_eq!(config.opt_level, 0);
        assert!(config.debug_info);
        assert!(config.incremental);
        assert!(config.skip_tests);
        assert!(config.dev_features.contains(&"dev".to_string()));
    }
    
    #[test]
    fn test_project_size_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Create src directory
        std::fs::create_dir_all(project_path.join("src")).unwrap();
        
        // Create Cargo.toml
        std::fs::write(
            project_path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\""
        ).unwrap();
        
        // Test small project
        let size = FastDevMode::analyze_project_size(project_path).unwrap();
        assert!(matches!(size, ProjectSize::Small));
        
        // Add some files to make it medium
        for i in 0..15 {
            std::fs::write(
                project_path.join(format!("src/file{}.rs", i)),
                format!("// File {}", i)
            ).unwrap();
        }
        
        let size = FastDevMode::analyze_project_size(project_path).unwrap();
        assert!(matches!(size, ProjectSize::Medium));
    }
    
    #[test]
    fn test_workspace_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Test non-workspace project
        std::fs::write(
            project_path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\""
        ).unwrap();
        
        assert!(!FastDevMode::has_workspace(project_path).unwrap());
        
        // Test workspace project
        std::fs::write(
            project_path.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crate1\", \"crate2\"]"
        ).unwrap();
        
        assert!(FastDevMode::has_workspace(project_path).unwrap());
    }
    
    #[test]
    fn test_wasm_target_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        
        // Test non-wasm project
        std::fs::write(
            project_path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\""
        ).unwrap();
        
        assert!(!FastDevMode::has_wasm_target(project_path).unwrap());
        
        // Test wasm project
        std::fs::write(
            project_path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n\n[dependencies]\nleptos = \"0.1\""
        ).unwrap();
        
        assert!(FastDevMode::has_wasm_target(project_path).unwrap());
    }
}
