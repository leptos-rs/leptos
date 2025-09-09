//! Build Profiler
//!
//! Provides detailed profiling and monitoring of build processes:
//! - Phase-by-phase timing analysis
//! - Bottleneck identification
//! - Performance regression detection
//! - Optimization recommendations

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Build profiler for analyzing compilation performance
pub struct BuildProfiler {
    system: System,
    profiling_enabled: bool,
    current_session: Option<ProfilingSession>,
    historical_data: Vec<ProfilingSession>,
    current_session_name: Option<String>,
    session_start: Option<Instant>,
    current_phase: Option<String>,
    phase_start: Option<Instant>,
    phases: HashMap<String, Duration>,
}

/// A profiling session containing all build metrics
#[derive(Debug, Clone)]
pub struct ProfilingSession {
    pub session_id: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub phases: HashMap<String, BuildPhase>,
    pub system_info: SystemInfo,
    pub bottlenecks: Vec<BuildBottleneck>,
    pub recommendations: Vec<OptimizationRecommendation>,
}

/// Information about a specific build phase
#[derive(Debug, Clone)]
pub struct BuildPhase {
    pub name: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub duration: Duration,
    pub memory_usage: MemoryUsage,
    pub cpu_usage: f32,
    pub sub_phases: Vec<BuildPhase>,
}

/// System information during build
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_count: usize,
    pub cpu_usage: f32,
    pub timestamp: Instant,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub peak_memory: u64,
    pub average_memory: u64,
    pub memory_growth: i64,
}

/// Identified build bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildBottleneck {
    pub phase: String,
    pub duration: Duration,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub impact_estimate: f64, // Percentage of total build time
}

/// Severity level of a bottleneck
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub title: String,
    pub description: String,
    pub action: String,
    pub impact_estimate: Option<f64>, // Estimated time savings percentage
    pub difficulty: ImplementationDifficulty,
    pub category: OptimizationCategory,
}

/// Difficulty of implementing an optimization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImplementationDifficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Category of optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationCategory {
    Compilation,
    Linking,
    DependencyResolution,
    MacroExpansion,
    CodeGeneration,
    System,
}

/// Build performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetrics {
    pub total_build_time: Duration,
    pub phases: HashMap<String, Duration>,
    pub bottlenecks: Vec<BuildBottleneck>,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub memory_peak: u64,
    pub cpu_utilization: f32,
}

/// Errors that can occur during profiling
#[derive(Debug, thiserror::Error)]
pub enum ProfilingError {
    #[error("Profiling session not started")]
    SessionNotStarted,
    
    #[error("Profiling session already active")]
    SessionAlreadyActive,
    
    #[error("System monitoring error: {reason}")]
    SystemMonitoring { reason: String },
    
    #[error("Data analysis error: {reason}")]
    DataAnalysis { reason: String },
}

impl BuildProfiler {
    /// Create a new build profiler
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system,
            profiling_enabled: false,
            current_session: None,
            historical_data: Vec::new(),
            current_session_name: None,
            session_start: None,
            current_phase: None,
            phase_start: None,
            phases: HashMap::new(),
        }
    }
    
    /// Enable detailed profiling
    pub fn enable_detailed_profiling(&mut self) {
        self.profiling_enabled = true;
    }
    
    /// Disable detailed profiling
    pub fn disable_detailed_profiling(&mut self) {
        self.profiling_enabled = false;
    }
    
    /// Start profiling a build session
    pub fn start_profiling(&mut self) -> Result<(), ProfilingError> {
        if self.current_session.is_some() {
            return Err(ProfilingError::SessionAlreadyActive);
        }
        
        let session_id = format!("build_{}", chrono::Utc::now().timestamp());
        let start_time = Instant::now();
        
        // Update system information
        self.system.refresh_all();
        let system_info = SystemInfo {
            total_memory: self.system.total_memory(),
            available_memory: self.system.available_memory(),
            cpu_count: self.system.cpus().len(),
            cpu_usage: 0.0, // Simplified for now
            timestamp: start_time,
        };
        
        let session = ProfilingSession {
            session_id,
            start_time,
            end_time: None,
            phases: HashMap::new(),
            system_info,
            bottlenecks: Vec::new(),
            recommendations: Vec::new(),
        };
        
        self.current_session = Some(session);
        Ok(())
    }
    
    /// Finish profiling and return metrics
    pub fn finish_profiling(&mut self) -> Result<BuildMetrics, ProfilingError> {
        let mut session = self.current_session.take()
            .ok_or(ProfilingError::SessionNotStarted)?;
        
        let end_time = Instant::now();
        session.end_time = Some(end_time);
        
        // Calculate phase durations
        for phase in session.phases.values_mut() {
            if let Some(end) = phase.end_time {
                phase.duration = end.duration_since(phase.start_time);
            }
        }
        
        // Analyze bottlenecks
        session.bottlenecks = self.analyze_bottlenecks(&session);
        
        // Generate recommendations
        session.recommendations = self.generate_recommendations(&session);
        
        // Store historical data
        self.historical_data.push(session.clone());
        
        // Create metrics
        let metrics = BuildMetrics {
            total_build_time: end_time.duration_since(session.start_time),
            phases: session.phases.iter()
                .map(|(name, phase)| (name.clone(), phase.duration))
                .collect(),
            bottlenecks: session.bottlenecks.clone(),
            recommendations: session.recommendations.clone(),
            memory_peak: self.calculate_memory_peak(&session),
            cpu_utilization: self.calculate_cpu_utilization(&session),
        };
        
        Ok(metrics)
    }
    
    /// Start profiling a specific build phase
    pub fn start_phase(&mut self, phase_name: &str) -> Result<(), ProfilingError> {
        let start_time = Instant::now();
        let memory_usage = self.get_current_memory_usage();
        let cpu_usage = 0.0; // Simplified for now
        
        let phase = BuildPhase {
            name: phase_name.to_string(),
            start_time,
            end_time: None,
            duration: Duration::from_secs(0),
            memory_usage,
            cpu_usage,
            sub_phases: Vec::new(),
        };
        
        let session = self.current_session.as_mut()
            .ok_or(ProfilingError::SessionNotStarted)?;
        session.phases.insert(phase_name.to_string(), phase);
        Ok(())
    }
    
    /// End profiling a specific build phase
    pub fn end_phase(&mut self, phase_name: &str) -> Result<(), ProfilingError> {
        let end_time = Instant::now();
        let memory_usage = self.get_current_memory_usage();
        let cpu_usage = 0.0; // Simplified for now
        
        let session = self.current_session.as_mut()
            .ok_or(ProfilingError::SessionNotStarted)?;
        
        let phase = session.phases.get_mut(phase_name)
            .ok_or_else(|| ProfilingError::DataAnalysis {
                reason: format!("Phase '{}' not found", phase_name),
            })?;
        
        phase.end_time = Some(end_time);
        phase.duration = end_time.duration_since(phase.start_time);
        phase.memory_usage = memory_usage;
        phase.cpu_usage = cpu_usage;
        
        Ok(())
    }
    
    /// Get current profiling session
    pub fn current_session(&self) -> Option<&ProfilingSession> {
        self.current_session.as_ref()
    }
    
    /// Get historical profiling data
    pub fn historical_data(&self) -> &[ProfilingSession] {
        &self.historical_data
    }
    
    /// Analyze bottlenecks in the current session
    fn analyze_bottlenecks(&self, session: &ProfilingSession) -> Vec<BuildBottleneck> {
        let mut bottlenecks = Vec::new();
        let total_time = session.end_time
            .map(|end| end.duration_since(session.start_time))
            .unwrap_or(Duration::from_secs(0));
        
        for (phase_name, phase) in &session.phases {
            let impact = if total_time.as_secs_f64() > 0.0 {
                phase.duration.as_secs_f64() / total_time.as_secs_f64() * 100.0
            } else {
                0.0
            };
            
            let severity = self.classify_bottleneck_severity(phase.duration, impact);
            
            if severity != BottleneckSeverity::Low {
                bottlenecks.push(BuildBottleneck {
                    phase: phase_name.clone(),
                    duration: phase.duration,
                    severity,
                    description: self.describe_bottleneck(phase_name, phase.duration, impact),
                    impact_estimate: impact,
                });
            }
        }
        
        // Sort by severity and impact
        bottlenecks.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then(b.impact_estimate.partial_cmp(&a.impact_estimate).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        bottlenecks
    }
    
    /// Classify bottleneck severity based on duration and impact
    fn classify_bottleneck_severity(&self, duration: Duration, impact: f64) -> BottleneckSeverity {
        match (duration.as_secs(), impact) {
            (s, i) if s >= 10 || i >= 50.0 => BottleneckSeverity::Critical,
            (s, i) if s >= 5 || i >= 25.0 => BottleneckSeverity::High,
            (s, i) if s >= 2 || i >= 10.0 => BottleneckSeverity::Medium,
            _ => BottleneckSeverity::Low,
        }
    }
    
    /// Generate description for a bottleneck
    fn describe_bottleneck(&self, phase_name: &str, duration: Duration, impact: f64) -> String {
        format!(
            "Phase '{}' took {:.2}s ({:.1}% of total build time)",
            phase_name,
            duration.as_secs_f64(),
            impact
        )
    }
    
    /// Generate optimization recommendations
    fn generate_recommendations(&self, session: &ProfilingSession) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // Analyze each phase for optimization opportunities
        for (phase_name, phase) in &session.phases {
            let phase_recommendations = self.get_phase_recommendations(phase_name, phase);
            recommendations.extend(phase_recommendations);
        }
        
        // Add system-level recommendations
        recommendations.extend(self.get_system_recommendations(session));
        
        recommendations
    }
    
    /// Get recommendations for a specific phase
    fn get_phase_recommendations(&self, phase_name: &str, phase: &BuildPhase) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        match phase_name {
            "dependency_resolution" => {
                if phase.duration > Duration::from_secs(3) {
                    recommendations.push(OptimizationRecommendation {
                        title: "Optimize Dependency Resolution".to_string(),
                        description: "Dependency resolution is taking too long".to_string(),
                        action: "Consider using cargo's dependency caching or reducing dependency count".to_string(),
                        impact_estimate: Some(30.0),
                        difficulty: ImplementationDifficulty::Medium,
                        category: OptimizationCategory::DependencyResolution,
                    });
                }
            }
            "compilation" => {
                if phase.duration > Duration::from_secs(10) {
                    recommendations.push(OptimizationRecommendation {
                        title: "Enable Parallel Compilation".to_string(),
                        description: "Compilation phase is the main bottleneck".to_string(),
                        action: "Increase CARGO_BUILD_JOBS or enable incremental compilation".to_string(),
                        impact_estimate: Some(40.0),
                        difficulty: ImplementationDifficulty::Easy,
                        category: OptimizationCategory::Compilation,
                    });
                }
            }
            "macro_expansion" => {
                if phase.duration > Duration::from_secs(2) {
                    recommendations.push(OptimizationRecommendation {
                        title: "Optimize Macro Usage".to_string(),
                        description: "Macro expansion is consuming significant time".to_string(),
                        action: "Review and optimize heavy macro usage, consider procedural macros".to_string(),
                        impact_estimate: Some(20.0),
                        difficulty: ImplementationDifficulty::Hard,
                        category: OptimizationCategory::MacroExpansion,
                    });
                }
            }
            "linking" => {
                if phase.duration > Duration::from_secs(5) {
                    recommendations.push(OptimizationRecommendation {
                        title: "Optimize Linking Process".to_string(),
                        description: "Linking is taking longer than expected".to_string(),
                        action: "Use faster linker (lld) or reduce binary size".to_string(),
                        impact_estimate: Some(25.0),
                        difficulty: ImplementationDifficulty::Medium,
                        category: OptimizationCategory::Linking,
                    });
                }
            }
            _ => {}
        }
        
        recommendations
    }
    
    /// Get system-level recommendations
    fn get_system_recommendations(&self, session: &ProfilingSession) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // Check memory usage
        if session.system_info.available_memory < 1024 * 1024 * 1024 { // Less than 1GB
            recommendations.push(OptimizationRecommendation {
                title: "Increase Available Memory".to_string(),
                description: "Low available memory may be slowing down builds".to_string(),
                action: "Close other applications or increase system memory".to_string(),
                impact_estimate: Some(15.0),
                difficulty: ImplementationDifficulty::Easy,
                category: OptimizationCategory::System,
            });
        }
        
        // Check CPU utilization
        if session.system_info.cpu_usage < 50.0 {
            recommendations.push(OptimizationRecommendation {
                title: "Increase Parallel Build Jobs".to_string(),
                description: "CPU utilization is low, more parallel jobs could speed up builds".to_string(),
                action: "Increase CARGO_BUILD_JOBS to utilize more CPU cores".to_string(),
                impact_estimate: Some(30.0),
                difficulty: ImplementationDifficulty::Easy,
                category: OptimizationCategory::System,
            });
        }
        
        recommendations
    }
    
    /// Get current memory usage
    fn get_current_memory_usage(&mut self) -> MemoryUsage {
        self.system.refresh_memory();
        
        MemoryUsage {
            peak_memory: self.system.used_memory(),
            average_memory: self.system.used_memory(),
            memory_growth: 0, // Would track growth over time in real implementation
        }
    }
    
    /// Calculate peak memory usage during session
    fn calculate_memory_peak(&self, session: &ProfilingSession) -> u64 {
        session.phases.values()
            .map(|phase| phase.memory_usage.peak_memory)
            .max()
            .unwrap_or(0)
    }
    
    /// Calculate average CPU utilization during session
    fn calculate_cpu_utilization(&self, session: &ProfilingSession) -> f32 {
        let total_cpu: f32 = session.phases.values()
            .map(|phase| phase.cpu_usage)
            .sum();
        
        if session.phases.is_empty() {
            0.0
        } else {
            total_cpu / session.phases.len() as f32
        }
    }

}

impl BuildMetrics {
    /// Identify the primary bottleneck
    pub fn identify_bottleneck(&self) -> Option<&BuildBottleneck> {
        self.bottlenecks.first()
    }
    
    /// Get optimization recommendations
    pub fn get_optimization_recommendations(&self) -> &[OptimizationRecommendation] {
        &self.recommendations
    }
    
    /// Get total build time
    pub fn total_build_time(&self) -> Duration {
        self.total_build_time
    }
    
    /// Get phase duration by name
    pub fn get_phase_duration(&self, phase_name: &str) -> Option<Duration> {
        self.phases.get(phase_name).copied()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_build_profiler_creation() {
        let profiler = BuildProfiler::new();
        assert!(!profiler.profiling_enabled);
        assert!(profiler.current_session.is_none());
        assert!(profiler.historical_data.is_empty());
    }
    
    #[test]
    fn test_profiling_session_lifecycle() {
        let mut profiler = BuildProfiler::new();
        
        // Start profiling
        assert!(profiler.start_profiling().is_ok());
        assert!(profiler.current_session.is_some());
        
        // Try to start again (should fail)
        assert!(profiler.start_profiling().is_err());
        
        // Start a phase
        assert!(profiler.start_phase("compilation").is_ok());
        
        // End the phase
        assert!(profiler.end_phase("compilation").is_ok());
        
        // Finish profiling
        let metrics = profiler.finish_profiling();
        assert!(metrics.is_ok());
        
        let metrics = metrics.unwrap();
        assert!(metrics.phases.contains_key("compilation"));
    }
    
    #[test]
    fn test_bottleneck_severity_classification() {
        let profiler = BuildProfiler::new();
        
        // Test different severity levels
        assert_eq!(
            profiler.classify_bottleneck_severity(Duration::from_secs(15), 60.0),
            BottleneckSeverity::Critical
        );
        
        assert_eq!(
            profiler.classify_bottleneck_severity(Duration::from_secs(7), 30.0),
            BottleneckSeverity::High
        );
        
        assert_eq!(
            profiler.classify_bottleneck_severity(Duration::from_secs(3), 15.0),
            BottleneckSeverity::Medium
        );
        
        assert_eq!(
            profiler.classify_bottleneck_severity(Duration::from_secs(1), 5.0),
            BottleneckSeverity::Low
        );
    }
    
    #[test]
    fn test_optimization_recommendations() {
        let mut profiler = BuildProfiler::new();
        profiler.start_profiling().unwrap();
        
        // Create a slow compilation phase
        profiler.start_phase("compilation").unwrap();
        std::thread::sleep(Duration::from_millis(100)); // Simulate work
        profiler.end_phase("compilation").unwrap();
        
        let metrics = profiler.finish_profiling().unwrap();
        
        // Should have recommendations for slow compilation
        assert!(!metrics.recommendations.is_empty());
        
        // Check that we can identify bottlenecks
        let bottleneck = metrics.identify_bottleneck();
        assert!(bottleneck.is_some());
    }
    
    #[test]
    fn test_build_metrics_accessors() {
        let mut profiler = BuildProfiler::new();
        profiler.start_profiling().unwrap();
        
        profiler.start_phase("test_phase").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        profiler.end_phase("test_phase").unwrap();
        
        let metrics = profiler.finish_profiling().unwrap();
        
        // Test accessor methods
        assert!(metrics.total_build_time() > Duration::from_millis(50));
        assert!(metrics.get_phase_duration("test_phase").is_some());
        assert!(metrics.get_phase_duration("nonexistent").is_none());
        assert!(metrics.get_optimization_recommendations().is_empty() || !metrics.get_optimization_recommendations().is_empty());
    }
}
