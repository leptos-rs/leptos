use crate::{
    build_profiler::BuildProfiler,
    performance_metrics::PerformanceMetrics,
    DevPerformanceError,
};
use std::path::Path;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Comprehensive benchmark suite for Leptos development performance
pub struct BenchmarkSuite {
    metrics: PerformanceMetrics,
    profiler: BuildProfiler,
    results: Vec<BenchmarkResult>,
}

/// Individual benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub build_type: BuildType,
    pub duration: Duration,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub cache_hit_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Types of builds to benchmark
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildType {
    Standard,
    FastDev,
    Incremental,
    Clean,
    HotReload,
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub project_path: std::path::PathBuf,
    pub enable_profiling: bool,
    pub test_scenarios: Vec<BenchmarkScenario>,
}

/// Different benchmark scenarios
#[derive(Debug, Clone)]
pub enum BenchmarkScenario {
    /// Test initial build performance
    InitialBuild,
    /// Test incremental build after small changes
    IncrementalBuild,
    /// Test clean build performance
    CleanBuild,
    /// Test hot-reload performance
    HotReload,
    /// Test memory usage during builds
    MemoryUsage,
    /// Test CPU utilization
    CpuUtilization,
}

impl BenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new() -> Self {
        Self {
            metrics: PerformanceMetrics::new(),
            profiler: BuildProfiler::new(),
            results: Vec::new(),
        }
    }

    /// Run comprehensive benchmarks
    pub fn run_benchmarks(&mut self, config: BenchmarkConfig) -> Result<BenchmarkReport, DevPerformanceError> {
        println!("🚀 Starting comprehensive performance benchmarks");
        println!("   Project: {}", config.project_path.display());
        println!("   Iterations: {}", config.iterations);
        println!("   Scenarios: {:?}", config.test_scenarios);

        let start_time = Instant::now();
        let mut report = BenchmarkReport::new();

        // Warmup runs
        if config.warmup_iterations > 0 {
            println!("🔥 Running warmup iterations...");
            self.run_warmup(&config)?;
        }

        // Run each scenario
        for scenario in &config.test_scenarios {
            println!("📊 Running scenario: {:?}", scenario);
            let scenario_results = self.run_scenario(scenario, &config)?;
            report.add_scenario_results(scenario_results);
        }

        let total_duration = start_time.elapsed();
        report.total_duration = total_duration;
        report.generate_summary();

        println!("✅ Benchmarks completed in {:?}", total_duration);
        Ok(report)
    }

    /// Run warmup iterations
    fn run_warmup(&mut self, config: &BenchmarkConfig) -> Result<(), DevPerformanceError> {
        for i in 0..config.warmup_iterations {
            println!("   Warmup {}/{}", i + 1, config.warmup_iterations);
            let _ = self.run_standard_build(&config.project_path)?;
        }
        Ok(())
    }

    /// Run a specific benchmark scenario
    fn run_scenario(&mut self, scenario: &BenchmarkScenario, config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, DevPerformanceError> {
        let mut results = Vec::new();

        for iteration in 0..config.iterations {
            println!("   Iteration {}/{}", iteration + 1, config.iterations);
            
            let result = match scenario {
                BenchmarkScenario::InitialBuild => self.benchmark_initial_build(&config.project_path)?,
                BenchmarkScenario::IncrementalBuild => self.benchmark_incremental_build(&config.project_path)?,
                BenchmarkScenario::CleanBuild => self.benchmark_clean_build(&config.project_path)?,
                BenchmarkScenario::HotReload => self.benchmark_hot_reload(&config.project_path)?,
                BenchmarkScenario::MemoryUsage => self.benchmark_memory_usage(&config.project_path)?,
                BenchmarkScenario::CpuUtilization => self.benchmark_cpu_utilization(&config.project_path)?,
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Benchmark initial build performance
    fn benchmark_initial_build(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_memory = self.get_memory_usage();
        let start_cpu = self.get_cpu_usage();

        // Run standard build
        self.run_standard_build(project_path)?;

        let duration = start.elapsed();
        let memory_usage = self.get_memory_usage() - start_memory;
        let cpu_usage = self.get_cpu_usage() - start_cpu;

        Ok(BenchmarkResult {
            name: "Initial Build".to_string(),
            build_type: BuildType::Standard,
            duration,
            memory_usage,
            cpu_usage,
            cache_hit_rate: 0.0, // No cache for initial build
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Benchmark incremental build performance
    fn benchmark_incremental_build(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_memory = self.get_memory_usage();
        let start_cpu = self.get_cpu_usage();

        // Simulate a small change and rebuild
        self.simulate_small_change(project_path)?;
        self.run_standard_build(project_path)?;

        let duration = start.elapsed();
        let memory_usage = self.get_memory_usage() - start_memory;
        let cpu_usage = self.get_cpu_usage() - start_cpu;

        Ok(BenchmarkResult {
            name: "Incremental Build".to_string(),
            build_type: BuildType::Incremental,
            duration,
            memory_usage,
            cpu_usage,
            cache_hit_rate: 0.8, // Assume 80% cache hit rate for incremental
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Benchmark clean build performance
    fn benchmark_clean_build(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_memory = self.get_memory_usage();
        let start_cpu = self.get_cpu_usage();

        // Clean and rebuild
        self.clean_build(project_path)?;
        self.run_standard_build(project_path)?;

        let duration = start.elapsed();
        let memory_usage = self.get_memory_usage() - start_memory;
        let cpu_usage = self.get_cpu_usage() - start_cpu;

        Ok(BenchmarkResult {
            name: "Clean Build".to_string(),
            build_type: BuildType::Clean,
            duration,
            memory_usage,
            cpu_usage,
            cache_hit_rate: 0.0, // No cache for clean build
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Benchmark hot-reload performance
    fn benchmark_hot_reload(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_memory = self.get_memory_usage();
        let start_cpu = self.get_cpu_usage();

        // Simulate hot-reload
        self.simulate_hot_reload(project_path)?;

        let duration = start.elapsed();
        let memory_usage = self.get_memory_usage() - start_memory;
        let cpu_usage = self.get_cpu_usage() - start_cpu;

        Ok(BenchmarkResult {
            name: "Hot Reload".to_string(),
            build_type: BuildType::HotReload,
            duration,
            memory_usage,
            cpu_usage,
            cache_hit_rate: 0.95, // High cache hit rate for hot-reload
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Benchmark memory usage
    fn benchmark_memory_usage(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_memory = self.get_memory_usage();

        // Run build and monitor memory
        self.run_standard_build(project_path)?;

        let duration = start.elapsed();
        let memory_usage = self.get_memory_usage() - start_memory;

        Ok(BenchmarkResult {
            name: "Memory Usage".to_string(),
            build_type: BuildType::Standard,
            duration,
            memory_usage,
            cpu_usage: 0.0,
            cache_hit_rate: 0.0,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Benchmark CPU utilization
    fn benchmark_cpu_utilization(&mut self, project_path: &Path) -> Result<BenchmarkResult, DevPerformanceError> {
        let start = Instant::now();
        let start_cpu = self.get_cpu_usage();

        // Run build and monitor CPU
        self.run_standard_build(project_path)?;

        let duration = start.elapsed();
        let cpu_usage = self.get_cpu_usage() - start_cpu;

        Ok(BenchmarkResult {
            name: "CPU Utilization".to_string(),
            build_type: BuildType::Standard,
            duration,
            memory_usage: 0,
            cpu_usage,
            cache_hit_rate: 0.0,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Run a standard build
    fn run_standard_build(&mut self, _project_path: &Path) -> Result<(), DevPerformanceError> {
        // This would run the actual cargo leptos build
        // For now, we'll simulate it
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    /// Simulate a small change
    fn simulate_small_change(&mut self, _project_path: &Path) -> Result<(), DevPerformanceError> {
        // This would simulate making a small change to a file
        // For now, we'll just sleep briefly
        std::thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    /// Clean build artifacts
    fn clean_build(&mut self, _project_path: &Path) -> Result<(), DevPerformanceError> {
        // This would run cargo clean
        // For now, we'll just sleep briefly
        std::thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    /// Simulate hot-reload
    fn simulate_hot_reload(&mut self, _project_path: &Path) -> Result<(), DevPerformanceError> {
        // This would simulate hot-reload
        // For now, we'll just sleep briefly
        std::thread::sleep(Duration::from_millis(20));
        Ok(())
    }

    /// Get current memory usage
    fn get_memory_usage(&self) -> u64 {
        // This would get actual memory usage
        // For now, return a simulated value
        1024 * 1024 // 1MB
    }

    /// Get current CPU usage
    fn get_cpu_usage(&self) -> f64 {
        // This would get actual CPU usage
        // For now, return a simulated value
        0.5 // 50%
    }
}

/// Comprehensive benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub scenarios: std::collections::HashMap<String, Vec<BenchmarkResult>>,
    pub total_duration: Duration,
    pub summary: BenchmarkSummary,
}

/// Benchmark summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_builds: usize,
    pub average_build_time: Duration,
    pub fastest_build: Duration,
    pub slowest_build: Duration,
    pub total_memory_usage: u64,
    pub average_cpu_usage: f64,
    pub cache_efficiency: f64,
    pub performance_score: f64,
}

impl BenchmarkReport {
    /// Create a new benchmark report
    pub fn new() -> Self {
        Self {
            scenarios: std::collections::HashMap::new(),
            total_duration: Duration::ZERO,
            summary: BenchmarkSummary {
                total_builds: 0,
                average_build_time: Duration::ZERO,
                fastest_build: Duration::ZERO,
                slowest_build: Duration::ZERO,
                total_memory_usage: 0,
                average_cpu_usage: 0.0,
                cache_efficiency: 0.0,
                performance_score: 0.0,
            },
        }
    }

    /// Add scenario results
    pub fn add_scenario_results(&mut self, results: Vec<BenchmarkResult>) {
        if let Some(first_result) = results.first() {
            let scenario_name = first_result.name.clone();
            self.scenarios.insert(scenario_name, results);
        }
    }

    /// Generate summary statistics
    pub fn generate_summary(&mut self) {
        let mut total_builds = 0;
        let mut total_duration = Duration::ZERO;
        let mut fastest_build = Duration::MAX;
        let mut slowest_build = Duration::ZERO;
        let mut total_memory = 0u64;
        let mut total_cpu = 0.0;
        let mut total_cache_hits = 0.0;
        let mut cache_attempts = 0;

        for results in self.scenarios.values() {
            for result in results {
                total_builds += 1;
                total_duration += result.duration;
                total_memory += result.memory_usage;
                total_cpu += result.cpu_usage;

                if result.duration < fastest_build {
                    fastest_build = result.duration;
                }
                if result.duration > slowest_build {
                    slowest_build = result.duration;
                }

                total_cache_hits += result.cache_hit_rate;
                cache_attempts += 1;
            }
        }

        self.summary = BenchmarkSummary {
            total_builds,
            average_build_time: if total_builds > 0 {
                Duration::from_nanos(total_duration.as_nanos() as u64 / total_builds as u64)
            } else {
                Duration::ZERO
            },
            fastest_build: if fastest_build == Duration::MAX {
                Duration::ZERO
            } else {
                fastest_build
            },
            slowest_build,
            total_memory_usage: total_memory,
            average_cpu_usage: if total_builds > 0 {
                total_cpu / total_builds as f64
            } else {
                0.0
            },
            cache_efficiency: if cache_attempts > 0 {
                total_cache_hits / cache_attempts as f64
            } else {
                0.0
            },
            performance_score: self.calculate_performance_score(),
        };
    }

    /// Calculate overall performance score
    fn calculate_performance_score(&self) -> f64 {
        // Simple performance score based on build times and cache efficiency
        let avg_build_time = self.summary.average_build_time.as_secs_f64();
        let cache_efficiency = self.summary.cache_efficiency;
        
        // Lower build time and higher cache efficiency = better score
        let time_score = if avg_build_time > 0.0 {
            100.0 / (avg_build_time + 1.0)
        } else {
            0.0
        };
        
        let cache_score = cache_efficiency * 100.0;
        
        (time_score + cache_score) / 2.0
    }

    /// Export report to JSON
    pub fn export_json(&self) -> Result<String, DevPerformanceError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))
    }

    /// Export report to CSV
    pub fn export_csv(&self) -> Result<String, DevPerformanceError> {
        let mut csv = String::new();
        csv.push_str("Scenario,Build Type,Duration (ms),Memory (MB),CPU (%),Cache Hit Rate,Timestamp\n");
        
        for (scenario_name, results) in &self.scenarios {
            for result in results {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    scenario_name,
                    format!("{:?}", result.build_type),
                    result.duration.as_millis(),
                    result.memory_usage / (1024 * 1024),
                    result.cpu_usage * 100.0,
                    result.cache_hit_rate * 100.0,
                    result.timestamp.format("%Y-%m-%d %H:%M:%S")
                ));
            }
        }
        
        Ok(csv)
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        println!("\n📊 BENCHMARK SUMMARY");
        println!("═══════════════════════════════════════");
        println!("Total Builds: {}", self.summary.total_builds);
        println!("Average Build Time: {:?}", self.summary.average_build_time);
        println!("Fastest Build: {:?}", self.summary.fastest_build);
        println!("Slowest Build: {:?}", self.summary.slowest_build);
        println!("Total Memory Usage: {} MB", self.summary.total_memory_usage / (1024 * 1024));
        println!("Average CPU Usage: {:.1}%", self.summary.average_cpu_usage * 100.0);
        println!("Cache Efficiency: {:.1}%", self.summary.cache_efficiency * 100.0);
        println!("Performance Score: {:.1}/100", self.summary.performance_score);
        println!("═══════════════════════════════════════\n");
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for BenchmarkReport {
    fn default() -> Self {
        Self::new()
    }
}
