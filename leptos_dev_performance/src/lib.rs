//! Leptos Development Performance Optimizations
//!
//! This crate addresses the critical P0 development performance issue identified in the roadmap:
//! "30+ second compilation times" that are blocking Leptos adoption.
//!
//! Key components:
//! - FastDevMode: Optimized development builds
//! - IncrementalCompiler: Smart incremental compilation
//! - HotReloadManager: Reliable hot-reload system  
//! - BuildProfiler: Performance monitoring and bottleneck identification

mod build_profiler;
mod fast_dev_mode;
mod hot_reload_manager;
mod incremental_compiler;
mod performance_metrics;
mod benchmark_suite;
mod performance_validator;
mod performance_reporter;

pub use build_profiler::{BuildProfiler, BuildPhase, OptimizationRecommendation};
pub use fast_dev_mode::{FastDevMode, FastDevError};
pub use hot_reload_manager::{HotReloadManager, HotReloadError};
pub use incremental_compiler::{IncrementalCompiler, CompilationResult, IncrementalError};
pub use performance_metrics::{PerformanceMetrics, PerformanceTargets, BuildBottleneck};
pub use benchmark_suite::{BenchmarkSuite, BenchmarkConfig, BenchmarkScenario, BenchmarkReport, BenchmarkResult, BenchmarkSummary, BuildType};
pub use performance_validator::{PerformanceValidator, PerformanceThresholds, ValidationReport, ValidationStatus, ValidationRule};
pub use performance_reporter::{PerformanceReporter, PerformanceReport, ReportFormat};

// Performance targets are now defined in performance_metrics module

/// Error types for development performance operations
#[derive(Debug, thiserror::Error)]
pub enum DevPerformanceError {
    #[error("Fast development mode error: {0}")]
    FastDev(#[from] FastDevError),
    
    #[error("Hot reload error: {0}")]
    HotReload(#[from] HotReloadError),
    
    #[error("Incremental compilation error: {0}")]
    Incremental(#[from] IncrementalError),
    
    #[error("Build profiling error: {message}")]
    Profiling { message: String },
    
    #[error("Performance target not met: {target} took {actual:?}, max allowed {max:?}")]
    PerformanceTarget {
        target: String,
        actual: std::time::Duration,
        max: std::time::Duration,
    },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Profiling error: {0}")]
    ProfilingError(#[from] build_profiler::ProfilingError),
}

pub type Result<T> = std::result::Result<T, DevPerformanceError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_targets_are_reasonable() {
        // Ensure our performance targets are achievable but meaningful
        let targets = PerformanceTargets::default();
        assert!(targets.initial_build_max.as_secs() > 10);
        assert!(targets.initial_build_max.as_secs() < 30);
        
        assert!(targets.incremental_build_max.as_secs() > 1);
        assert!(targets.incremental_build_max.as_secs() < 10);
        
        assert!(targets.hot_reload_success_rate > 0.9);
        assert!(targets.hot_reload_success_rate <= 1.0);
    }
}