use crate::{
    benchmark_suite::{BenchmarkResult, BenchmarkReport},
    DevPerformanceError,
};
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Performance validation and regression testing
pub struct PerformanceValidator {
    baseline_results: Option<BenchmarkReport>,
    thresholds: PerformanceThresholds,
    validation_rules: Vec<ValidationRule>,
}

/// Performance thresholds for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub max_build_time: Duration,
    pub max_memory_usage: u64,
    pub min_cache_efficiency: f64,
    pub max_cpu_usage: f64,
    pub regression_tolerance: f64, // Percentage increase allowed
}

/// Validation rules
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Build time should not exceed threshold
    BuildTimeThreshold(Duration),
    /// Memory usage should not exceed threshold
    MemoryThreshold(u64),
    /// Cache efficiency should meet minimum
    CacheEfficiencyThreshold(f64),
    /// CPU usage should not exceed threshold
    CpuThreshold(f64),
    /// Performance regression tolerance
    RegressionTolerance(f64),
}

/// Validation result
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Pass,
    Fail(String),
    Warning(String),
}

/// Performance validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub overall_status: ValidationStatus,
    pub rule_results: Vec<RuleValidationResult>,
    pub regression_analysis: Option<RegressionAnalysis>,
    pub recommendations: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Overall validation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStatus {
    Pass,
    Fail,
    Warning,
}

/// Individual rule validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleValidationResult {
    pub rule_name: String,
    pub status: ValidationStatus,
    pub message: String,
    pub details: Option<String>,
}

/// Regression analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysis {
    pub has_regression: bool,
    pub regression_percentage: f64,
    pub affected_scenarios: Vec<String>,
    pub severity: RegressionSeverity,
}

/// Regression severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegressionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl PerformanceValidator {
    /// Create a new performance validator
    pub fn new() -> Self {
        Self {
            baseline_results: None,
            thresholds: PerformanceThresholds::default(),
            validation_rules: Vec::new(),
        }
    }

    /// Set baseline results for regression testing
    pub fn set_baseline(&mut self, baseline: BenchmarkReport) {
        self.baseline_results = Some(baseline);
    }

    /// Set performance thresholds
    pub fn set_thresholds(&mut self, thresholds: PerformanceThresholds) {
        self.thresholds = thresholds;
    }

    /// Add validation rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.validation_rules.push(rule);
    }

    /// Validate performance results
    pub fn validate(&self, results: &BenchmarkReport) -> Result<ValidationReport, DevPerformanceError> {
        println!("🔍 Validating performance results...");
        
        let mut rule_results = Vec::new();
        let mut overall_status = ValidationStatus::Pass;

        // Run built-in validations
        self.validate_build_times(results, &mut rule_results, &mut overall_status);
        self.validate_memory_usage(results, &mut rule_results, &mut overall_status);
        self.validate_cache_efficiency(results, &mut rule_results, &mut overall_status);
        self.validate_cpu_usage(results, &mut rule_results, &mut overall_status);

        // Run additional validation rules
        for (i, rule) in self.validation_rules.iter().enumerate() {
            let result = self.validate_rule(rule, results);
            rule_results.push(RuleValidationResult {
                rule_name: format!("Additional Rule {}", i + 1),
                status: match result {
                    ValidationResult::Pass => ValidationStatus::Pass,
                    ValidationResult::Fail(_) => ValidationStatus::Fail,
                    ValidationResult::Warning(_) => ValidationStatus::Warning,
                },
                message: match result {
                    ValidationResult::Pass => "Rule passed".to_string(),
                    ValidationResult::Fail(msg) => msg,
                    ValidationResult::Warning(msg) => msg,
                },
                details: None,
            });

            // Update overall status
            if rule_results.last().unwrap().status == ValidationStatus::Fail {
                overall_status = ValidationStatus::Fail;
            } else if rule_results.last().unwrap().status == ValidationStatus::Warning && overall_status == ValidationStatus::Pass {
                overall_status = ValidationStatus::Warning;
            }
        }

        // Perform regression analysis if baseline exists
        let regression_analysis = if let Some(baseline) = &self.baseline_results {
            Some(self.analyze_regression(baseline, results))
        } else {
            None
        };

        // Generate recommendations
        let recommendations = self.generate_recommendations(&rule_results, &regression_analysis);

        Ok(ValidationReport {
            overall_status,
            rule_results,
            regression_analysis,
            recommendations,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Validate build times
    fn validate_build_times(&self, results: &BenchmarkReport, rule_results: &mut Vec<RuleValidationResult>, overall_status: &mut ValidationStatus) {
        let avg_build_time = results.summary.average_build_time;
        
        if avg_build_time > self.thresholds.max_build_time {
            rule_results.push(RuleValidationResult {
                rule_name: "Build Time Threshold".to_string(),
                status: ValidationStatus::Fail,
                message: format!(
                    "Average build time ({:?}) exceeds threshold ({:?})",
                    avg_build_time, self.thresholds.max_build_time
                ),
                details: Some(format!("Threshold exceeded by {:?}", avg_build_time - self.thresholds.max_build_time)),
            });
            *overall_status = ValidationStatus::Fail;
        } else {
            rule_results.push(RuleValidationResult {
                rule_name: "Build Time Threshold".to_string(),
                status: ValidationStatus::Pass,
                message: format!("Average build time ({:?}) is within threshold", avg_build_time),
                details: None,
            });
        }
    }

    /// Validate memory usage
    fn validate_memory_usage(&self, results: &BenchmarkReport, rule_results: &mut Vec<RuleValidationResult>, overall_status: &mut ValidationStatus) {
        let total_memory = results.summary.total_memory_usage;
        
        if total_memory > self.thresholds.max_memory_usage {
            rule_results.push(RuleValidationResult {
                rule_name: "Memory Usage Threshold".to_string(),
                status: ValidationStatus::Fail,
                message: format!(
                    "Total memory usage ({} MB) exceeds threshold ({} MB)",
                    total_memory / (1024 * 1024),
                    self.thresholds.max_memory_usage / (1024 * 1024)
                ),
                details: Some(format!("Threshold exceeded by {} MB", (total_memory - self.thresholds.max_memory_usage) / (1024 * 1024))),
            });
            *overall_status = ValidationStatus::Fail;
        } else {
            rule_results.push(RuleValidationResult {
                rule_name: "Memory Usage Threshold".to_string(),
                status: ValidationStatus::Pass,
                message: format!("Total memory usage ({} MB) is within threshold", total_memory / (1024 * 1024)),
                details: None,
            });
        }
    }

    /// Validate cache efficiency
    fn validate_cache_efficiency(&self, results: &BenchmarkReport, rule_results: &mut Vec<RuleValidationResult>, overall_status: &mut ValidationStatus) {
        let cache_efficiency = results.summary.cache_efficiency;
        
        if cache_efficiency < self.thresholds.min_cache_efficiency {
            rule_results.push(RuleValidationResult {
                rule_name: "Cache Efficiency Threshold".to_string(),
                status: ValidationStatus::Warning,
                message: format!(
                    "Cache efficiency ({:.1}%) is below threshold ({:.1}%)",
                    cache_efficiency * 100.0,
                    self.thresholds.min_cache_efficiency * 100.0
                ),
                details: Some(format!("Threshold missed by {:.1}%", (self.thresholds.min_cache_efficiency - cache_efficiency) * 100.0)),
            });
            if *overall_status == ValidationStatus::Pass {
                *overall_status = ValidationStatus::Warning;
            }
        } else {
            rule_results.push(RuleValidationResult {
                rule_name: "Cache Efficiency Threshold".to_string(),
                status: ValidationStatus::Pass,
                message: format!("Cache efficiency ({:.1}%) meets threshold", cache_efficiency * 100.0),
                details: None,
            });
        }
    }

    /// Validate CPU usage
    fn validate_cpu_usage(&self, results: &BenchmarkReport, rule_results: &mut Vec<RuleValidationResult>, overall_status: &mut ValidationStatus) {
        let avg_cpu = results.summary.average_cpu_usage;
        
        if avg_cpu > self.thresholds.max_cpu_usage {
            rule_results.push(RuleValidationResult {
                rule_name: "CPU Usage Threshold".to_string(),
                status: ValidationStatus::Warning,
                message: format!(
                    "Average CPU usage ({:.1}%) exceeds threshold ({:.1}%)",
                    avg_cpu * 100.0,
                    self.thresholds.max_cpu_usage * 100.0
                ),
                details: Some(format!("Threshold exceeded by {:.1}%", (avg_cpu - self.thresholds.max_cpu_usage) * 100.0)),
            });
            if *overall_status == ValidationStatus::Pass {
                *overall_status = ValidationStatus::Warning;
            }
        } else {
            rule_results.push(RuleValidationResult {
                rule_name: "CPU Usage Threshold".to_string(),
                status: ValidationStatus::Pass,
                message: format!("Average CPU usage ({:.1}%) is within threshold", avg_cpu * 100.0),
                details: None,
            });
        }
    }

    /// Validate a rule
    fn validate_rule(&self, rule: &ValidationRule, results: &BenchmarkReport) -> ValidationResult {
        match rule {
            ValidationRule::BuildTimeThreshold(max_time) => {
                if results.summary.average_build_time > *max_time {
                    ValidationResult::Fail(format!(
                        "Build time ({:?}) exceeds threshold ({:?})",
                        results.summary.average_build_time, max_time
                    ))
                } else {
                    ValidationResult::Pass
                }
            }
            ValidationRule::MemoryThreshold(max_memory) => {
                if results.summary.total_memory_usage > *max_memory {
                    ValidationResult::Fail(format!(
                        "Memory usage ({} MB) exceeds threshold ({} MB)",
                        results.summary.total_memory_usage / (1024 * 1024),
                        max_memory / (1024 * 1024)
                    ))
                } else {
                    ValidationResult::Pass
                }
            }
            ValidationRule::CacheEfficiencyThreshold(min_efficiency) => {
                if results.summary.cache_efficiency < *min_efficiency {
                    ValidationResult::Warning(format!(
                        "Cache efficiency ({:.1}%) is below threshold ({:.1}%)",
                        results.summary.cache_efficiency * 100.0,
                        min_efficiency * 100.0
                    ))
                } else {
                    ValidationResult::Pass
                }
            }
            ValidationRule::CpuThreshold(max_cpu) => {
                if results.summary.average_cpu_usage > *max_cpu {
                    ValidationResult::Warning(format!(
                        "CPU usage ({:.1}%) exceeds threshold ({:.1}%)",
                        results.summary.average_cpu_usage * 100.0,
                        max_cpu * 100.0
                    ))
                } else {
                    ValidationResult::Pass
                }
            }
            ValidationRule::RegressionTolerance(tolerance) => {
                // This would be handled in regression analysis
                ValidationResult::Pass
            }
        }
    }

    /// Analyze performance regression
    fn analyze_regression(&self, baseline: &BenchmarkReport, current: &BenchmarkReport) -> RegressionAnalysis {
        let mut has_regression = false;
        let mut total_regression = 0.0;
        let mut regression_count = 0;
        let mut affected_scenarios = Vec::new();

        // Compare build times
        for (scenario_name, current_results) in &current.scenarios {
            if let Some(baseline_results) = baseline.scenarios.get(scenario_name) {
                if let (Some(current_avg), Some(baseline_avg)) = (
                    self.calculate_average_duration(current_results),
                    self.calculate_average_duration(baseline_results)
                ) {
                    let regression_percentage = ((current_avg.as_secs_f64() - baseline_avg.as_secs_f64()) / baseline_avg.as_secs_f64()) * 100.0;
                    
                    if regression_percentage > self.thresholds.regression_tolerance {
                        has_regression = true;
                        total_regression += regression_percentage;
                        regression_count += 1;
                        affected_scenarios.push(scenario_name.clone());
                    }
                }
            }
        }

        let avg_regression = if regression_count > 0 {
            total_regression / regression_count as f64
        } else {
            0.0
        };

        let severity = if avg_regression > 50.0 {
            RegressionSeverity::Critical
        } else if avg_regression > 25.0 {
            RegressionSeverity::High
        } else if avg_regression > 10.0 {
            RegressionSeverity::Medium
        } else {
            RegressionSeverity::Low
        };

        RegressionAnalysis {
            has_regression,
            regression_percentage: avg_regression,
            affected_scenarios,
            severity,
        }
    }

    /// Calculate average duration for a set of results
    fn calculate_average_duration(&self, results: &[BenchmarkResult]) -> Option<Duration> {
        if results.is_empty() {
            return None;
        }

        let total_nanos: u64 = results.iter()
            .map(|r| r.duration.as_nanos() as u64)
            .sum();
        
        Some(Duration::from_nanos(total_nanos / results.len() as u64))
    }

    /// Generate recommendations based on validation results
    fn generate_recommendations(&self, rule_results: &[RuleValidationResult], regression_analysis: &Option<RegressionAnalysis>) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Analyze rule results
        for result in rule_results {
            match result.status {
                ValidationStatus::Fail => {
                    match result.rule_name.as_str() {
                        "Build Time Threshold" => {
                            recommendations.push("Consider enabling fast development mode or optimizing build configuration".to_string());
                        }
                        "Memory Usage Threshold" => {
                            recommendations.push("Review memory usage patterns and consider reducing build parallelism".to_string());
                        }
                        _ => {}
                    }
                }
                ValidationStatus::Warning => {
                    match result.rule_name.as_str() {
                        "Cache Efficiency Threshold" => {
                            recommendations.push("Improve cache configuration and ensure incremental builds are working properly".to_string());
                        }
                        "CPU Usage Threshold" => {
                            recommendations.push("Consider reducing build parallelism or optimizing CPU-intensive operations".to_string());
                        }
                        _ => {}
                    }
                }
                ValidationStatus::Pass => {}
            }
        }

        // Analyze regression
        if let Some(regression) = regression_analysis {
            if regression.has_regression {
                match regression.severity {
                    RegressionSeverity::Critical => {
                        recommendations.push("CRITICAL: Immediate investigation required for performance regression".to_string());
                    }
                    RegressionSeverity::High => {
                        recommendations.push("HIGH: Performance regression detected, review recent changes".to_string());
                    }
                    RegressionSeverity::Medium => {
                        recommendations.push("MEDIUM: Monitor performance regression and consider optimization".to_string());
                    }
                    RegressionSeverity::Low => {
                        recommendations.push("LOW: Minor performance regression detected".to_string());
                    }
                }
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Performance is within acceptable thresholds".to_string());
        }

        recommendations
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_build_time: Duration::from_secs(30), // 30 seconds
            max_memory_usage: 1024 * 1024 * 1024, // 1GB
            min_cache_efficiency: 0.7, // 70%
            max_cpu_usage: 0.8, // 80%
            regression_tolerance: 10.0, // 10% increase allowed
        }
    }
}

impl Default for PerformanceValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common validation rules
impl PerformanceValidator {
    /// Create a build time threshold rule
    pub fn build_time_threshold(max_time: Duration) -> ValidationRule {
        ValidationRule::BuildTimeThreshold(max_time)
    }

    /// Create a memory usage threshold rule
    pub fn memory_threshold(max_memory: u64) -> ValidationRule {
        ValidationRule::MemoryThreshold(max_memory)
    }

    /// Create a cache efficiency threshold rule
    pub fn cache_efficiency_threshold(min_efficiency: f64) -> ValidationRule {
        ValidationRule::CacheEfficiencyThreshold(min_efficiency)
    }

    /// Create a CPU usage threshold rule
    pub fn cpu_threshold(max_cpu: f64) -> ValidationRule {
        ValidationRule::CpuThreshold(max_cpu)
    }

    /// Create a regression tolerance rule
    pub fn regression_tolerance(tolerance: f64) -> ValidationRule {
        ValidationRule::RegressionTolerance(tolerance)
    }
}

impl ValidationReport {
    /// Print validation report to console
    pub fn print_report(&self) {
        println!("\n🔍 PERFORMANCE VALIDATION REPORT");
        println!("═══════════════════════════════════════");
        println!("Overall Status: {:?}", self.overall_status);
        println!("Timestamp: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S"));
        
        println!("\n📋 Rule Results:");
        for result in &self.rule_results {
            let status_icon = match result.status {
                ValidationStatus::Pass => "✅",
                ValidationStatus::Warning => "⚠️",
                ValidationStatus::Fail => "❌",
            };
            println!("  {} {}: {}", status_icon, result.rule_name, result.message);
            if let Some(details) = &result.details {
                println!("    Details: {}", details);
            }
        }

        if let Some(regression) = &self.regression_analysis {
            println!("\n📈 Regression Analysis:");
            if regression.has_regression {
                println!("  ❌ Performance regression detected");
                println!("  Regression: {:.1}%", regression.regression_percentage);
                println!("  Severity: {:?}", regression.severity);
                println!("  Affected scenarios: {:?}", regression.affected_scenarios);
            } else {
                println!("  ✅ No significant performance regression");
            }
        }

        println!("\n💡 Recommendations:");
        for (i, recommendation) in self.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, recommendation);
        }
        
        println!("═══════════════════════════════════════\n");
    }

    /// Export validation report to JSON
    pub fn export_json(&self) -> Result<String, DevPerformanceError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
    }
}
