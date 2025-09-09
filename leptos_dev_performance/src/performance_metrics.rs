//! Performance Metrics and Monitoring
//!
//! Provides comprehensive performance tracking and analysis:
//! - Build time tracking and trends
//! - Performance target validation
//! - Regression detection
//! - Historical performance analysis

use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Performance metrics collector and analyzer
pub struct PerformanceMetrics {
    build_history: VecDeque<BuildRecord>,
    target_config: PerformanceTargets,
    regression_threshold: f64,
    max_history_size: usize,
}

/// Record of a single build performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    pub timestamp: DateTime<Utc>,
    pub build_type: BuildType,
    pub duration: Duration,
    pub modules_compiled: usize,
    pub memory_peak: u64,
    pub cpu_utilization: f32,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

/// Type of build performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuildType {
    Initial,
    Incremental,
    Full,
    Clean,
    Test,
}

/// Performance targets for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTargets {
    pub initial_build_max: Duration,
    pub incremental_build_max: Duration,
    pub hot_reload_max: Duration,
    pub hot_reload_success_rate: f64,
    pub max_regression_factor: f64,
}

/// Identified performance bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildBottleneck {
    pub phase: String,
    pub duration: Duration,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub impact_estimate: f64,
}

/// Severity of a performance bottleneck
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub trend_direction: TrendDirection,
    pub change_percentage: f64,
    pub confidence: f64,
    pub sample_size: usize,
    pub time_period: Duration,
}

/// Direction of performance trend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Degrading,
    Stable,
}

/// Performance regression detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    pub is_regression: bool,
    pub severity: RegressionSeverity,
    pub baseline_duration: Duration,
    pub current_duration: Duration,
    pub regression_factor: f64,
    pub recommendation: String,
}

/// Severity of performance regression
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegressionSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Performance summary for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub time_period: Duration,
    pub total_builds: usize,
    pub successful_builds: usize,
    pub average_build_time: Duration,
    pub median_build_time: Duration,
    pub p95_build_time: Duration,
    pub fastest_build: Duration,
    pub slowest_build: Duration,
    pub trend: PerformanceTrend,
    pub regressions: Vec<RegressionResult>,
}

/// Errors that can occur in performance metrics
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    #[error("Insufficient data for analysis: {reason}")]
    InsufficientData { reason: String },
    
    #[error("Invalid performance target: {target}")]
    InvalidTarget { target: String },
    
    #[error("Data analysis error: {reason}")]
    AnalysisError { reason: String },
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            initial_build_max: Duration::from_secs(15),
            incremental_build_max: Duration::from_secs(5),
            hot_reload_max: Duration::from_secs(2),
            hot_reload_success_rate: 0.95,
            max_regression_factor: 1.5,
        }
    }
}

impl PerformanceMetrics {
    /// Create a new performance metrics collector
    pub fn new() -> Self {
        Self::with_targets(PerformanceTargets::default())
    }
    
    /// Create with custom performance targets
    pub fn with_targets(targets: PerformanceTargets) -> Self {
        Self {
            build_history: VecDeque::new(),
            target_config: targets,
            regression_threshold: 1.2, // 20% regression threshold
            max_history_size: 1000, // Keep last 1000 builds
        }
    }
    
    /// Record a build performance
    pub fn record_build(&mut self, record: BuildRecord) {
        self.build_history.push_back(record);
        
        // Maintain history size limit
        if self.build_history.len() > self.max_history_size {
            self.build_history.pop_front();
        }
    }
    
    /// Record a build with basic information
    pub fn record_simple_build(
        &mut self,
        build_type: BuildType,
        duration: Duration,
        success: bool,
    ) {
        let record = BuildRecord {
            timestamp: Utc::now(),
            build_type,
            duration,
            modules_compiled: 0,
            memory_peak: 0,
            cpu_utilization: 0.0,
            success,
            metadata: HashMap::new(),
        };
        
        self.record_build(record);
    }
    
    /// Validate build performance against targets
    pub fn validate_performance(&self, build_type: &BuildType, duration: Duration) -> bool {
        match build_type {
            BuildType::Initial => duration <= self.target_config.initial_build_max,
            BuildType::Incremental => duration <= self.target_config.incremental_build_max,
            BuildType::Full => duration <= self.target_config.initial_build_max,
            BuildType::Clean => duration <= self.target_config.initial_build_max,
            BuildType::Test => duration <= Duration::from_secs(30), // Test-specific target
        }
    }
    
    /// Detect performance regressions
    pub fn detect_regressions(&self, lookback_period: Duration) -> Vec<RegressionResult> {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(lookback_period).unwrap_or_default();
        
        let recent_builds: Vec<&BuildRecord> = self.build_history
            .iter()
            .filter(|record| record.timestamp >= cutoff_time)
            .collect();
        
        if recent_builds.len() < 10 {
            return Vec::new(); // Need sufficient data
        }
        
        let mut regressions = Vec::new();
        
        // Group builds by type for comparison
        let mut builds_by_type: HashMap<BuildType, Vec<&BuildRecord>> = HashMap::new();
        for build in &recent_builds {
            builds_by_type.entry(build.build_type.clone()).or_default().push(build);
        }
        
        for (build_type, builds) in builds_by_type {
            if builds.len() < 5 {
                continue; // Need sufficient samples
            }
            
            // Calculate baseline (first half) vs current (second half)
            let split_point = builds.len() / 2;
            let baseline_builds = &builds[..split_point];
            let current_builds = &builds[split_point..];
            
            let baseline_avg = self.calculate_average_duration(baseline_builds);
            let current_avg = self.calculate_average_duration(current_builds);
            
            let regression_factor = if baseline_avg.as_secs_f64() > 0.0 {
                current_avg.as_secs_f64() / baseline_avg.as_secs_f64()
            } else {
                1.0
            };
            
            if regression_factor > self.regression_threshold {
                let severity = self.classify_regression_severity(regression_factor);
                let recommendation = self.generate_regression_recommendation(
                    &build_type,
                    baseline_avg,
                    current_avg,
                    regression_factor,
                );
                
                regressions.push(RegressionResult {
                    is_regression: true,
                    severity,
                    baseline_duration: baseline_avg,
                    current_duration: current_avg,
                    regression_factor,
                    recommendation,
                });
            }
        }
        
        regressions.sort_by(|a, b| b.severity.cmp(&a.severity));
        regressions
    }
    
    /// Analyze performance trends
    pub fn analyze_trends(&self, time_period: Duration) -> Result<PerformanceTrend, PerformanceError> {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(time_period).unwrap_or_default();
        
        let period_builds: Vec<&BuildRecord> = self.build_history
            .iter()
            .filter(|record| record.timestamp >= cutoff_time)
            .collect();
        
        if period_builds.len() < 5 {
            return Err(PerformanceError::InsufficientData {
                reason: format!("Need at least 5 builds, got {}", period_builds.len()),
            });
        }
        
        // Calculate trend using linear regression
        let trend_data = self.calculate_linear_trend(&period_builds);
        
        Ok(PerformanceTrend {
            trend_direction: self.classify_trend_direction(trend_data.slope),
            change_percentage: trend_data.change_percentage,
            confidence: trend_data.confidence,
            sample_size: period_builds.len(),
            time_period,
        })
    }
    
    /// Generate performance summary for a time period
    pub fn generate_summary(&self, time_period: Duration) -> Result<PerformanceSummary, PerformanceError> {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(time_period).unwrap_or_default();
        
        let period_builds: Vec<&BuildRecord> = self.build_history
            .iter()
            .filter(|record| record.timestamp >= cutoff_time)
            .collect();
        
        if period_builds.is_empty() {
            return Err(PerformanceError::InsufficientData {
                reason: "No builds found in the specified time period".to_string(),
            });
        }
        
        let successful_builds: Vec<&BuildRecord> = period_builds
            .iter()
            .filter(|build| build.success)
            .copied()
            .collect();
        
        let durations: Vec<Duration> = successful_builds
            .iter()
            .map(|build| build.duration)
            .collect();
        
        let average_build_time = self.calculate_average_duration(&successful_builds);
        let median_build_time = self.calculate_median_duration(durations.clone());
        let p95_build_time = self.calculate_percentile_duration(durations.clone(), 95);
        let fastest_build = durations.iter().min().copied().unwrap_or_default();
        let slowest_build = durations.iter().max().copied().unwrap_or_default();
        
        let trend = self.analyze_trends(time_period).unwrap_or_else(|_| PerformanceTrend {
            trend_direction: TrendDirection::Stable,
            change_percentage: 0.0,
            confidence: 0.0,
            sample_size: 0,
            time_period,
        });
        
        let regressions = self.detect_regressions(time_period);
        
        Ok(PerformanceSummary {
            time_period,
            total_builds: period_builds.len(),
            successful_builds: successful_builds.len(),
            average_build_time,
            median_build_time,
            p95_build_time,
            fastest_build,
            slowest_build,
            trend,
            regressions,
        })
    }
    
    /// Get build history
    pub fn get_build_history(&self) -> &VecDeque<BuildRecord> {
        &self.build_history
    }
    
    /// Get performance targets
    pub fn get_targets(&self) -> &PerformanceTargets {
        &self.target_config
    }
    
    /// Update performance targets
    pub fn update_targets(&mut self, targets: PerformanceTargets) {
        self.target_config = targets;
    }
    
    /// Calculate average duration from build records
    fn calculate_average_duration(&self, builds: &[&BuildRecord]) -> Duration {
        if builds.is_empty() {
            return Duration::from_secs(0);
        }
        
        let total_secs: f64 = builds
            .iter()
            .map(|build| build.duration.as_secs_f64())
            .sum();
        
        Duration::from_secs_f64(total_secs / builds.len() as f64)
    }
    
    /// Calculate median duration
    fn calculate_median_duration(&self, mut durations: Vec<Duration>) -> Duration {
        if durations.is_empty() {
            return Duration::from_secs(0);
        }
        
        durations.sort();
        let mid = durations.len() / 2;
        
        if durations.len() % 2 == 0 {
            let avg = (durations[mid - 1].as_secs_f64() + durations[mid].as_secs_f64()) / 2.0;
            Duration::from_secs_f64(avg)
        } else {
            durations[mid]
        }
    }
    
    /// Calculate percentile duration
    fn calculate_percentile_duration(&self, mut durations: Vec<Duration>, percentile: u8) -> Duration {
        if durations.is_empty() {
            return Duration::from_secs(0);
        }
        
        durations.sort();
        let index = ((percentile as f64 / 100.0) * (durations.len() - 1) as f64).round() as usize;
        durations[index.min(durations.len() - 1)]
    }
    
    /// Calculate linear trend using simple linear regression
    fn calculate_linear_trend(&self, builds: &[&BuildRecord]) -> TrendData {
        if builds.len() < 2 {
            return TrendData {
                slope: 0.0,
                change_percentage: 0.0,
                confidence: 0.0,
            };
        }
        
        let n = builds.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        
        for (i, build) in builds.iter().enumerate() {
            let x = i as f64;
            let y = build.duration.as_secs_f64();
            
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let first_duration = builds.first().unwrap().duration.as_secs_f64();
        let last_duration = builds.last().unwrap().duration.as_secs_f64();
        let change_percentage = if first_duration > 0.0 {
            ((last_duration - first_duration) / first_duration) * 100.0
        } else {
            0.0
        };
        
        // Simple confidence calculation based on R-squared
        let y_mean = sum_y / n;
        let mut ss_tot = 0.0;
        let mut ss_res = 0.0;
        
        for (i, build) in builds.iter().enumerate() {
            let x = i as f64;
            let y = build.duration.as_secs_f64();
            let y_pred = slope * x + (sum_y - slope * sum_x) / n;
            
            ss_tot += (y - y_mean).powi(2);
            ss_res += (y - y_pred).powi(2);
        }
        
        let confidence = if ss_tot > 0.0 {
            (1.0 - ss_res / ss_tot).max(0.0).min(1.0)
        } else {
            0.0
        };
        
        TrendData {
            slope,
            change_percentage,
            confidence,
        }
    }
    
    /// Classify trend direction based on slope
    fn classify_trend_direction(&self, slope: f64) -> TrendDirection {
        const THRESHOLD: f64 = 0.1; // 10% change threshold
        
        if slope > THRESHOLD {
            TrendDirection::Degrading
        } else if slope < -THRESHOLD {
            TrendDirection::Improving
        } else {
            TrendDirection::Stable
        }
    }
    
    /// Classify regression severity
    fn classify_regression_severity(&self, regression_factor: f64) -> RegressionSeverity {
        match regression_factor {
            f if f >= 2.0 => RegressionSeverity::Critical,
            f if f >= 1.5 => RegressionSeverity::Major,
            f if f >= 1.3 => RegressionSeverity::Moderate,
            _ => RegressionSeverity::Minor,
        }
    }
    
    /// Generate recommendation for regression
    fn generate_regression_recommendation(
        &self,
        build_type: &BuildType,
        baseline: Duration,
        current: Duration,
        factor: f64,
    ) -> String {
        format!(
            "{} builds have regressed by {:.1}% (from {:.1}s to {:.1}s). Consider investigating recent changes or optimizing build configuration.",
            match build_type {
                BuildType::Initial => "Initial",
                BuildType::Incremental => "Incremental",
                BuildType::Full => "Full",
                BuildType::Clean => "Clean",
                BuildType::Test => "Test",
            },
            (factor - 1.0) * 100.0,
            baseline.as_secs_f64(),
            current.as_secs_f64()
        )
    }
}

/// Internal trend calculation data
#[derive(Debug)]
struct TrendData {
    slope: f64,
    change_percentage: f64,
    confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new();
        assert!(metrics.get_build_history().is_empty());
        assert_eq!(metrics.get_targets().initial_build_max, Duration::from_secs(15));
    }
    
    #[test]
    fn test_build_recording() {
        let mut metrics = PerformanceMetrics::new();
        
        let record = BuildRecord {
            timestamp: Utc::now(),
            build_type: BuildType::Incremental,
            duration: Duration::from_secs(3),
            modules_compiled: 5,
            memory_peak: 1024 * 1024 * 100, // 100MB
            cpu_utilization: 75.0,
            success: true,
            metadata: HashMap::new(),
        };
        
        metrics.record_build(record);
        assert_eq!(metrics.get_build_history().len(), 1);
    }
    
    #[test]
    fn test_performance_validation() {
        let metrics = PerformanceMetrics::new();
        
        // Test valid performance
        assert!(metrics.validate_performance(&BuildType::Incremental, Duration::from_secs(3)));
        assert!(metrics.validate_performance(&BuildType::Initial, Duration::from_secs(10)));
        
        // Test invalid performance
        assert!(!metrics.validate_performance(&BuildType::Incremental, Duration::from_secs(10)));
        assert!(!metrics.validate_performance(&BuildType::Initial, Duration::from_secs(20)));
    }
    
    #[test]
    fn test_regression_detection() {
        let mut metrics = PerformanceMetrics::new();
        
        // Add baseline builds (fast)
        for i in 0..5 {
            let record = BuildRecord {
                timestamp: Utc::now() - chrono::Duration::hours(2) + chrono::Duration::minutes(i),
                build_type: BuildType::Incremental,
                duration: Duration::from_secs(2),
                modules_compiled: 5,
                memory_peak: 0,
                cpu_utilization: 0.0,
                success: true,
                metadata: HashMap::new(),
            };
            metrics.record_build(record);
        }
        
        // Add current builds (slow)
        for i in 0..5 {
            let record = BuildRecord {
                timestamp: Utc::now() - chrono::Duration::minutes(30) + chrono::Duration::minutes(i),
                build_type: BuildType::Incremental,
                duration: Duration::from_secs(4), // 2x slower
                modules_compiled: 5,
                memory_peak: 0,
                cpu_utilization: 0.0,
                success: true,
                metadata: HashMap::new(),
            };
            metrics.record_build(record);
        }
        
        let regressions = metrics.detect_regressions(Duration::from_secs(3600)); // 1 hour
        assert!(!regressions.is_empty());
        assert!(regressions[0].is_regression);
        assert!(regressions[0].regression_factor > 1.5);
    }
    
    #[test]
    fn test_performance_summary() {
        let mut metrics = PerformanceMetrics::new();
        
        // Add some test builds
        for i in 0..10 {
            let record = BuildRecord {
                timestamp: Utc::now() - chrono::Duration::minutes(10 - i),
                build_type: BuildType::Incremental,
                duration: Duration::from_secs((2 + i) as u64),
                modules_compiled: 5,
                memory_peak: 0,
                cpu_utilization: 0.0,
                success: true,
                metadata: HashMap::new(),
            };
            metrics.record_build(record);
        }
        
        let summary = metrics.generate_summary(Duration::from_secs(3600));
        assert!(summary.is_ok());
        
        let summary = summary.unwrap();
        assert_eq!(summary.total_builds, 10);
        assert_eq!(summary.successful_builds, 10);
        assert!(summary.average_build_time > Duration::from_secs(0));
    }
    
    #[test]
    fn test_trend_analysis() {
        let mut metrics = PerformanceMetrics::new();
        
        // Add builds with improving trend
        for i in 0..10 {
            let record = BuildRecord {
                timestamp: Utc::now() - chrono::Duration::minutes(10 - i),
                build_type: BuildType::Incremental,
                duration: Duration::from_secs((10 - i) as u64), // Getting faster
                modules_compiled: 5,
                memory_peak: 0,
                cpu_utilization: 0.0,
                success: true,
                metadata: HashMap::new(),
            };
            metrics.record_build(record);
        }
        
        let trend = metrics.analyze_trends(Duration::from_secs(3600));
        assert!(trend.is_ok());
        
        let trend = trend.unwrap();
        assert_eq!(trend.trend_direction, TrendDirection::Improving);
        assert!(trend.change_percentage < 0.0); // Negative = improving
    }
}
