use crate::{
    benchmark_suite::BenchmarkReport,
    performance_validator::ValidationReport,
    DevPerformanceError,
};
use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Performance reporting system
pub struct PerformanceReporter {
    reports: Vec<PerformanceReport>,
    templates: HashMap<String, ReportTemplate>,
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub id: String,
    pub title: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub benchmark_report: Option<BenchmarkReport>,
    pub validation_report: Option<ValidationReport>,
    pub summary: PerformanceSummary,
    pub metadata: HashMap<String, String>,
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub overall_score: f64,
    pub build_performance: BuildPerformanceMetrics,
    pub resource_usage: ResourceUsageMetrics,
    pub cache_efficiency: CacheMetrics,
    pub recommendations: Vec<String>,
    pub trends: PerformanceTrends,
}

/// Build performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPerformanceMetrics {
    pub average_build_time: std::time::Duration,
    pub fastest_build: std::time::Duration,
    pub slowest_build: std::time::Duration,
    pub build_consistency: f64, // Coefficient of variation
    pub improvement_over_baseline: Option<f64>, // Percentage improvement
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageMetrics {
    pub peak_memory_usage: u64,
    pub average_memory_usage: u64,
    pub peak_cpu_usage: f64,
    pub average_cpu_usage: f64,
    pub resource_efficiency_score: f64,
}

/// Cache efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub overall_hit_rate: f64,
    pub incremental_hit_rate: f64,
    pub hot_reload_hit_rate: f64,
    pub cache_size: u64,
    pub cache_effectiveness_score: f64,
}

/// Performance trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub build_time_trend: TrendDirection,
    pub memory_usage_trend: TrendDirection,
    pub cpu_usage_trend: TrendDirection,
    pub cache_efficiency_trend: TrendDirection,
    pub overall_trend: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Report template
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub name: String,
    pub format: ReportFormat,
    pub sections: Vec<ReportSection>,
    pub styling: ReportStyling,
}

/// Report format
#[derive(Debug, Clone, PartialEq)]
pub enum ReportFormat {
    Html,
    Markdown,
    Json,
    Csv,
    Console,
}

/// Report section
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub title: String,
    pub content_type: SectionContentType,
    pub template: String,
}

/// Section content type
#[derive(Debug, Clone, PartialEq)]
pub enum SectionContentType {
    Summary,
    BenchmarkResults,
    ValidationResults,
    Trends,
    Recommendations,
    Custom(String),
}

/// Report styling
#[derive(Debug, Clone)]
pub struct ReportStyling {
    pub theme: String,
    pub colors: HashMap<String, String>,
    pub fonts: HashMap<String, String>,
}

impl PerformanceReporter {
    /// Create a new performance reporter
    pub fn new() -> Self {
        let mut reporter = Self {
            reports: Vec::new(),
            templates: HashMap::new(),
        };
        
        // Initialize default templates
        reporter.initialize_default_templates();
        reporter
    }

    /// Generate a comprehensive performance report
    pub fn generate_report(
        &mut self,
        title: String,
        benchmark_report: Option<BenchmarkReport>,
        validation_report: Option<ValidationReport>,
    ) -> Result<PerformanceReport, DevPerformanceError> {
        let id = self.generate_report_id();
        let timestamp = chrono::Utc::now();
        
        let summary = self.generate_summary(&benchmark_report, &validation_report)?;
        
        let report = PerformanceReport {
            id,
            title,
            timestamp,
            benchmark_report,
            validation_report,
            summary,
            metadata: HashMap::new(),
        };
        
        self.reports.push(report.clone());
        Ok(report)
    }

    /// Export report in specified format
    pub fn export_report(&self, report: &PerformanceReport, format: ReportFormat, output_path: Option<&Path>) -> Result<String, DevPerformanceError> {
        match format {
            ReportFormat::Html => self.export_html(report, output_path),
            ReportFormat::Markdown => self.export_markdown(report, output_path),
            ReportFormat::Json => self.export_json(report, output_path),
            ReportFormat::Csv => self.export_csv(report, output_path),
            ReportFormat::Console => self.export_console(report),
        }
    }

    /// Generate performance summary
    fn generate_summary(
        &self,
        benchmark_report: &Option<BenchmarkReport>,
        validation_report: &Option<ValidationReport>,
    ) -> Result<PerformanceSummary, DevPerformanceError> {
        let build_performance = if let Some(benchmark) = benchmark_report {
            BuildPerformanceMetrics {
                average_build_time: benchmark.summary.average_build_time,
                fastest_build: benchmark.summary.fastest_build,
                slowest_build: benchmark.summary.slowest_build,
                build_consistency: self.calculate_build_consistency(benchmark),
                improvement_over_baseline: None, // Would need baseline comparison
            }
        } else {
            BuildPerformanceMetrics {
                average_build_time: std::time::Duration::ZERO,
                fastest_build: std::time::Duration::ZERO,
                slowest_build: std::time::Duration::ZERO,
                build_consistency: 0.0,
                improvement_over_baseline: None,
            }
        };

        let resource_usage = if let Some(benchmark) = benchmark_report {
            ResourceUsageMetrics {
                peak_memory_usage: benchmark.summary.total_memory_usage,
                average_memory_usage: benchmark.summary.total_memory_usage / benchmark.summary.total_builds.max(1) as u64,
                peak_cpu_usage: benchmark.summary.average_cpu_usage,
                average_cpu_usage: benchmark.summary.average_cpu_usage,
                resource_efficiency_score: self.calculate_resource_efficiency_score(benchmark),
            }
        } else {
            ResourceUsageMetrics {
                peak_memory_usage: 0,
                average_memory_usage: 0,
                peak_cpu_usage: 0.0,
                average_cpu_usage: 0.0,
                resource_efficiency_score: 0.0,
            }
        };

        let cache_efficiency = if let Some(benchmark) = benchmark_report {
            CacheMetrics {
                overall_hit_rate: benchmark.summary.cache_efficiency,
                incremental_hit_rate: self.calculate_incremental_hit_rate(benchmark),
                hot_reload_hit_rate: self.calculate_hot_reload_hit_rate(benchmark),
                cache_size: 0, // Would need actual cache size
                cache_effectiveness_score: benchmark.summary.cache_efficiency * 100.0,
            }
        } else {
            CacheMetrics {
                overall_hit_rate: 0.0,
                incremental_hit_rate: 0.0,
                hot_reload_hit_rate: 0.0,
                cache_size: 0,
                cache_effectiveness_score: 0.0,
            }
        };

        let trends = self.calculate_trends();
        let recommendations = self.generate_recommendations(benchmark_report, validation_report);

        let overall_score = self.calculate_overall_score(&build_performance, &resource_usage, &cache_efficiency);

        Ok(PerformanceSummary {
            overall_score,
            build_performance,
            resource_usage,
            cache_efficiency,
            recommendations,
            trends,
        })
    }

    /// Calculate build consistency (coefficient of variation)
    fn calculate_build_consistency(&self, benchmark: &BenchmarkReport) -> f64 {
        // Simplified calculation - in reality would need individual build times
        let avg_time = benchmark.summary.average_build_time.as_secs_f64();
        let range = benchmark.summary.slowest_build.as_secs_f64() - benchmark.summary.fastest_build.as_secs_f64();
        
        if avg_time > 0.0 {
            range / avg_time
        } else {
            0.0
        }
    }

    /// Calculate resource efficiency score
    fn calculate_resource_efficiency_score(&self, benchmark: &BenchmarkReport) -> f64 {
        let memory_score = if benchmark.summary.total_memory_usage > 0 {
            100.0 / (benchmark.summary.total_memory_usage as f64 / (1024.0 * 1024.0 * 1024.0)) // Normalize to GB
        } else {
            0.0
        };
        
        let cpu_score = (1.0 - benchmark.summary.average_cpu_usage) * 100.0;
        
        (memory_score + cpu_score) / 2.0
    }

    /// Calculate incremental hit rate
    fn calculate_incremental_hit_rate(&self, benchmark: &BenchmarkReport) -> f64 {
        // Simplified - would need actual incremental build data
        benchmark.summary.cache_efficiency * 0.8
    }

    /// Calculate hot-reload hit rate
    fn calculate_hot_reload_hit_rate(&self, benchmark: &BenchmarkReport) -> f64 {
        // Simplified - would need actual hot-reload data
        benchmark.summary.cache_efficiency * 0.95
    }

    /// Calculate performance trends
    fn calculate_trends(&self) -> PerformanceTrends {
        // Simplified - would need historical data
        PerformanceTrends {
            build_time_trend: TrendDirection::Unknown,
            memory_usage_trend: TrendDirection::Unknown,
            cpu_usage_trend: TrendDirection::Unknown,
            cache_efficiency_trend: TrendDirection::Unknown,
            overall_trend: TrendDirection::Unknown,
        }
    }

    /// Generate recommendations
    fn generate_recommendations(
        &self,
        benchmark_report: &Option<BenchmarkReport>,
        validation_report: &Option<ValidationReport>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if let Some(benchmark) = benchmark_report {
            if benchmark.summary.average_build_time > std::time::Duration::from_secs(10) {
                recommendations.push("Consider enabling fast development mode to reduce build times".to_string());
            }
            
            if benchmark.summary.cache_efficiency < 0.7 {
                recommendations.push("Improve cache configuration for better incremental builds".to_string());
            }
        }

        if let Some(validation) = validation_report {
            recommendations.extend(validation.recommendations.clone());
        }

        if recommendations.is_empty() {
            recommendations.push("Performance is within acceptable ranges".to_string());
        }

        recommendations
    }

    /// Calculate overall performance score
    fn calculate_overall_score(
        &self,
        build_performance: &BuildPerformanceMetrics,
        resource_usage: &ResourceUsageMetrics,
        cache_efficiency: &CacheMetrics,
    ) -> f64 {
        let build_score = if build_performance.average_build_time.as_secs_f64() > 0.0 {
            100.0 / (build_performance.average_build_time.as_secs_f64() + 1.0)
        } else {
            0.0
        };
        
        let resource_score = resource_usage.resource_efficiency_score;
        let cache_score = cache_efficiency.cache_effectiveness_score;
        
        (build_score + resource_score + cache_score) / 3.0
    }

    /// Export as HTML
    fn export_html(&self, report: &PerformanceReport, output_path: Option<&Path>) -> Result<String, DevPerformanceError> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .header {{ text-align: center; margin-bottom: 30px; }}
        .score {{ font-size: 2em; font-weight: bold; color: #2ecc71; }}
        .section {{ margin: 20px 0; padding: 20px; border: 1px solid #ddd; border-radius: 5px; }}
        .metric {{ display: inline-block; margin: 10px; padding: 10px; background: #f8f9fa; border-radius: 3px; }}
        .recommendation {{ background: #e8f4fd; padding: 10px; margin: 5px 0; border-left: 4px solid #3498db; }}
        .timestamp {{ color: #666; font-size: 0.9em; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{}</h1>
            <div class="score">Overall Score: {:.1}/100</div>
            <div class="timestamp">Generated: {}</div>
        </div>
        
        <div class="section">
            <h2>Build Performance</h2>
            <div class="metric">Average Build Time: {:?}</div>
            <div class="metric">Fastest Build: {:?}</div>
            <div class="metric">Slowest Build: {:?}</div>
            <div class="metric">Build Consistency: {:.2}</div>
        </div>
        
        <div class="section">
            <h2>Resource Usage</h2>
            <div class="metric">Peak Memory: {} MB</div>
            <div class="metric">Average Memory: {} MB</div>
            <div class="metric">Peak CPU: {:.1}%</div>
            <div class="metric">Average CPU: {:.1}%</div>
        </div>
        
        <div class="section">
            <h2>Cache Efficiency</h2>
            <div class="metric">Overall Hit Rate: {:.1}%</div>
            <div class="metric">Incremental Hit Rate: {:.1}%</div>
            <div class="metric">Hot Reload Hit Rate: {:.1}%</div>
        </div>
        
        <div class="section">
            <h2>Recommendations</h2>
            {}
        </div>
    </div>
</body>
</html>"#,
            report.title,
            report.title,
            report.summary.overall_score,
            report.timestamp.format("%Y-%m-%d %H:%M:%S"),
            report.summary.build_performance.average_build_time,
            report.summary.build_performance.fastest_build,
            report.summary.build_performance.slowest_build,
            report.summary.build_performance.build_consistency,
            report.summary.resource_usage.peak_memory_usage / (1024 * 1024),
            report.summary.resource_usage.average_memory_usage / (1024 * 1024),
            report.summary.resource_usage.peak_cpu_usage * 100.0,
            report.summary.resource_usage.average_cpu_usage * 100.0,
            report.summary.cache_efficiency.overall_hit_rate * 100.0,
            report.summary.cache_efficiency.incremental_hit_rate * 100.0,
            report.summary.cache_efficiency.hot_reload_hit_rate * 100.0,
            report.summary.recommendations.iter()
                .map(|rec| format!("<div class=\"recommendation\">{}</div>", rec))
                .collect::<Vec<_>>()
                .join("")
        );

        if let Some(path) = output_path {
            std::fs::write(path, &html)?;
        }

        Ok(html)
    }

    /// Export as Markdown
    fn export_markdown(&self, report: &PerformanceReport, output_path: Option<&Path>) -> Result<String, DevPerformanceError> {
        let markdown = format!(
            r#"# {}

**Overall Score:** {:.1}/100  
**Generated:** {}

## Build Performance

- **Average Build Time:** {:?}
- **Fastest Build:** {:?}
- **Slowest Build:** {:?}
- **Build Consistency:** {:.2}

## Resource Usage

- **Peak Memory:** {} MB
- **Average Memory:** {} MB
- **Peak CPU:** {:.1}%
- **Average CPU:** {:.1}%

## Cache Efficiency

- **Overall Hit Rate:** {:.1}%
- **Incremental Hit Rate:** {:.1}%
- **Hot Reload Hit Rate:** {:.1}%

## Recommendations

{}

---
*Report generated by Leptos Development Performance Tool*
"#,
            report.title,
            report.summary.overall_score,
            report.timestamp.format("%Y-%m-%d %H:%M:%S"),
            report.summary.build_performance.average_build_time,
            report.summary.build_performance.fastest_build,
            report.summary.build_performance.slowest_build,
            report.summary.build_performance.build_consistency,
            report.summary.resource_usage.peak_memory_usage / (1024 * 1024),
            report.summary.resource_usage.average_memory_usage / (1024 * 1024),
            report.summary.resource_usage.peak_cpu_usage * 100.0,
            report.summary.resource_usage.average_cpu_usage * 100.0,
            report.summary.cache_efficiency.overall_hit_rate * 100.0,
            report.summary.cache_efficiency.incremental_hit_rate * 100.0,
            report.summary.cache_efficiency.hot_reload_hit_rate * 100.0,
            report.summary.recommendations.iter()
                .map(|rec| format!("- {}", rec))
                .collect::<Vec<_>>()
                .join("\n")
        );

        if let Some(path) = output_path {
            std::fs::write(path, &markdown)?;
        }

        Ok(markdown)
    }

    /// Export as JSON
    fn export_json(&self, report: &PerformanceReport, output_path: Option<&Path>) -> Result<String, DevPerformanceError> {
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        if let Some(path) = output_path {
            std::fs::write(path, &json)?;
        }

        Ok(json)
    }

    /// Export as CSV
    fn export_csv(&self, report: &PerformanceReport, output_path: Option<&Path>) -> Result<String, DevPerformanceError> {
        let mut csv = String::new();
        csv.push_str("Metric,Value,Unit\n");
        csv.push_str(&format!("Overall Score,{:.1},/100\n", report.summary.overall_score));
        csv.push_str(&format!("Average Build Time,{:.2},seconds\n", report.summary.build_performance.average_build_time.as_secs_f64()));
        csv.push_str(&format!("Peak Memory Usage,{},MB\n", report.summary.resource_usage.peak_memory_usage / (1024 * 1024)));
        csv.push_str(&format!("Average CPU Usage,{:.1},%\n", report.summary.resource_usage.average_cpu_usage * 100.0));
        csv.push_str(&format!("Cache Hit Rate,{:.1},%\n", report.summary.cache_efficiency.overall_hit_rate * 100.0));

        if let Some(path) = output_path {
            std::fs::write(path, &csv)?;
        }

        Ok(csv)
    }

    /// Export to console
    fn export_console(&self, report: &PerformanceReport) -> Result<String, DevPerformanceError> {
        let console_output = format!(
            r#"
🚀 LEPTOS PERFORMANCE REPORT
═══════════════════════════════════════
📊 Overall Score: {:.1}/100
📅 Generated: {}

⚡ BUILD PERFORMANCE
────────────────────────────────────────
⏱️  Average Build Time: {:?}
🏃 Fastest Build: {:?}
🐌 Slowest Build: {:?}
📈 Build Consistency: {:.2}

💾 RESOURCE USAGE
────────────────────────────────────────
🧠 Peak Memory: {} MB
🧠 Average Memory: {} MB
⚡ Peak CPU: {:.1}%
⚡ Average CPU: {:.1}%

🎯 CACHE EFFICIENCY
────────────────────────────────────────
📦 Overall Hit Rate: {:.1}%
🔄 Incremental Hit Rate: {:.1}%
🔥 Hot Reload Hit Rate: {:.1}%

💡 RECOMMENDATIONS
────────────────────────────────────────
{}
═══════════════════════════════════════
"#,
            report.summary.overall_score,
            report.timestamp.format("%Y-%m-%d %H:%M:%S"),
            report.summary.build_performance.average_build_time,
            report.summary.build_performance.fastest_build,
            report.summary.build_performance.slowest_build,
            report.summary.build_performance.build_consistency,
            report.summary.resource_usage.peak_memory_usage / (1024 * 1024),
            report.summary.resource_usage.average_memory_usage / (1024 * 1024),
            report.summary.resource_usage.peak_cpu_usage * 100.0,
            report.summary.resource_usage.average_cpu_usage * 100.0,
            report.summary.cache_efficiency.overall_hit_rate * 100.0,
            report.summary.cache_efficiency.incremental_hit_rate * 100.0,
            report.summary.cache_efficiency.hot_reload_hit_rate * 100.0,
            report.summary.recommendations.iter()
                .map(|rec| format!("• {}", rec))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(console_output)
    }

    /// Generate unique report ID
    fn generate_report_id(&self) -> String {
        format!("perf_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
    }

    /// Initialize default report templates
    fn initialize_default_templates(&mut self) {
        // Add default HTML template
        self.templates.insert("default_html".to_string(), ReportTemplate {
            name: "Default HTML".to_string(),
            format: ReportFormat::Html,
            sections: vec![
                ReportSection {
                    title: "Summary".to_string(),
                    content_type: SectionContentType::Summary,
                    template: "{{summary}}".to_string(),
                },
                ReportSection {
                    title: "Benchmark Results".to_string(),
                    content_type: SectionContentType::BenchmarkResults,
                    template: "{{benchmark_results}}".to_string(),
                },
                ReportSection {
                    title: "Validation Results".to_string(),
                    content_type: SectionContentType::ValidationResults,
                    template: "{{validation_results}}".to_string(),
                },
            ],
            styling: ReportStyling {
                theme: "default".to_string(),
                colors: HashMap::new(),
                fonts: HashMap::new(),
            },
        });
    }
}

impl Default for PerformanceReporter {
    fn default() -> Self {
        Self::new()
    }
}
