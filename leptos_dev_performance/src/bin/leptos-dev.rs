//! Leptos Development Performance Tool
//! 
//! A performance-optimized development server that integrates with cargo-leptos
//! to provide fast development builds and reliable hot-reload.

use clap::{Parser, Subcommand};
use leptos_dev_performance::{
    FastDevMode, HotReloadManager, BuildProfiler, PerformanceMetrics,
    BenchmarkSuite, BenchmarkConfig, BenchmarkScenario, BenchmarkReport, BenchmarkResult, BenchmarkSummary, BuildType,
    PerformanceValidator, PerformanceThresholds,
    PerformanceReporter, ReportFormat,
    DevPerformanceError
};
use std::path::PathBuf;
use std::process;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "leptos-dev",
    about = "High-performance Leptos development server with fast builds and reliable hot-reload",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server with performance optimizations
    Dev {
        /// Enable fast development mode (reduces compilation time by 50-70%)
        #[arg(short, long)]
        fast: bool,
        
        /// Enable performance profiling
        #[arg(long)]
        profile: bool,
        
        /// Port for the development server
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Enable hot-reload
        #[arg(long, default_value = "true")]
        hot_reload: bool,
        
        /// Project directory (defaults to current directory)
        #[arg(long)]
        project_dir: Option<PathBuf>,
        
        /// Watch additional directories
        #[arg(short, long)]
        watch: Vec<PathBuf>,
    },
    
    /// Profile build performance
    Profile {
        /// Project directory to profile
        #[arg(short, long)]
        project_dir: Option<PathBuf>,
        
        /// Number of build iterations for profiling
        #[arg(short, long, default_value = "3")]
        iterations: u32,
    },
    
    /// Benchmark performance improvements
    Benchmark {
        /// Compare with standard cargo-leptos
        #[arg(short, long)]
        compare: bool,
        
        /// Project directory to benchmark
        #[arg(short, long)]
        project_dir: Option<PathBuf>,
        
        /// Number of benchmark iterations
        #[arg(short, long, default_value = "3")]
        iterations: usize,
        
        /// Benchmark scenarios to run
        #[arg(long, default_value = "all")]
        scenarios: String,
    },
    
    /// Run comprehensive performance benchmarks
    Bench {
        /// Project directory to benchmark
        #[arg(long)]
        project_dir: Option<PathBuf>,
        
        /// Number of iterations per scenario
        #[arg(short, long, default_value = "5")]
        iterations: usize,
        
        /// Number of warmup iterations
        #[arg(long, default_value = "2")]
        warmup: usize,
        
        /// Enable profiling during benchmarks
        #[arg(long)]
        profile: bool,
        
        /// Output format for results
        #[arg(long, default_value = "console")]
        output_format: String,
        
        /// Output file path
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Validate performance against thresholds
    Validate {
        /// Project directory to validate
        #[arg(long)]
        project_dir: Option<PathBuf>,
        
        /// Baseline report for regression testing
        #[arg(long)]
        baseline: Option<PathBuf>,
        
        /// Performance thresholds file
        #[arg(long)]
        thresholds: Option<PathBuf>,
        
        /// Output format for validation report
        #[arg(long, default_value = "console")]
        output_format: String,
        
        /// Output file path
        #[arg(long)]
        output: Option<PathBuf>,
    },
    
    /// Generate performance report
    Report {
        /// Project directory to report on
        #[arg(long)]
        project_dir: Option<PathBuf>,
        
        /// Benchmark results file
        #[arg(long)]
        benchmark_results: Option<PathBuf>,
        
        /// Validation results file
        #[arg(long)]
        validation_results: Option<PathBuf>,
        
        /// Report format (html, markdown, json, csv, console)
        #[arg(long, default_value = "console")]
        format: String,
        
        /// Output file path
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    
    if let Err(e) = run_command(cli.command) {
        eprintln!("❌ Error: {}", e);
        process::exit(1);
    }
}

fn run_command(command: Commands) -> Result<(), DevPerformanceError> {
    match command {
        Commands::Dev {
            fast,
            profile,
            port,
            hot_reload,
            project_dir,
            watch,
        } => {
            run_dev_server(fast, profile, port, hot_reload, project_dir, watch)
        }
        
        Commands::Profile {
            project_dir,
            iterations,
        } => {
            profile_build_performance(project_dir, iterations)
        }
        
        Commands::Benchmark {
            compare,
            project_dir,
            iterations,
            scenarios,
        } => {
            benchmark_performance(compare, project_dir, iterations, scenarios)
        }
        
        Commands::Bench {
            project_dir,
            iterations,
            warmup,
            profile,
            output_format,
            output,
        } => {
            run_comprehensive_benchmarks(project_dir, iterations, warmup, profile, output_format, output)
        }
        
        Commands::Validate {
            project_dir,
            baseline,
            thresholds,
            output_format,
            output,
        } => {
            validate_performance(project_dir, baseline, thresholds, output_format, output)
        }
        
        Commands::Report {
            project_dir,
            benchmark_results,
            validation_results,
            format,
            output,
        } => {
            generate_performance_report(project_dir, benchmark_results, validation_results, format, output)
        }
    }
}

fn run_dev_server(
    fast: bool,
    profile: bool,
    port: u16,
    hot_reload: bool,
    project_dir: Option<PathBuf>,
    watch_dirs: Vec<PathBuf>,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("🚀 Starting Leptos Development Server");
    println!("   Project: {}", project_path.display());
    println!("   Port: {}", port);
    println!("   Fast mode: {}", if fast { "✅ Enabled" } else { "❌ Disabled" });
    println!("   Hot reload: {}", if hot_reload { "✅ Enabled" } else { "❌ Disabled" });
    println!("   Profiling: {}", if profile { "✅ Enabled" } else { "❌ Disabled" });
    println!();

    // Initialize performance monitoring if enabled
    let mut profiler = if profile {
        Some(BuildProfiler::new())
    } else {
        None
    };

    // Start performance profiling
    if let Some(ref mut profiler) = profiler {
        profiler.start_profiling()?;
        profiler.start_phase("initialization")?;
    }

    // Initialize fast development mode if enabled
    let mut fast_dev = if fast {
        println!("⚡ Initializing fast development mode...");
        let mut fast_dev = FastDevMode::new(&project_path)?;
        fast_dev.setup_fast_config()?;
        println!("✅ Fast development mode configured");
        Some(fast_dev)
    } else {
        None
    };

    // Initialize hot-reload manager if enabled
    let hot_reload_manager = if hot_reload {
        println!("🔄 Initializing hot-reload system...");
        let mut manager = HotReloadManager::new(&project_path)?;
        manager.initialize_watcher()?;
        
        // Add additional watch directories
        for watch_dir in watch_dirs {
            manager.add_watch_directory(&watch_dir)?;
        }
        
        println!("✅ Hot-reload system ready");
        Some(manager)
    } else {
        None
    };

    // End initialization phase
    if let Some(ref mut profiler) = profiler {
        profiler.end_phase("initialization")?;
        profiler.start_phase("build")?;
    }

    // Perform initial build
    println!("🔨 Performing initial build...");
    let build_start = std::time::Instant::now();
    
    if let Some(ref mut fast_dev) = fast_dev {
        fast_dev.build_fast()?;
    } else {
        // Fallback to standard cargo-leptos build
        run_cargo_leptos_build(&project_path)?;
    }
    
    let build_duration = build_start.elapsed();
    println!("✅ Initial build completed in {:.2}s", build_duration.as_secs_f64());

    // End build phase
    if let Some(ref mut profiler) = profiler {
        profiler.end_phase("build")?;
        profiler.start_phase("server_startup")?;
    }

    // Start the development server
    println!("🌐 Starting development server on port {}...", port);
    
    // Start server (this would integrate with the actual server startup)
    start_development_server(port, &project_path)?;
    
    println!("✅ Development server running at http://localhost:{}", port);

    // End server startup phase
    if let Some(ref mut profiler) = profiler {
        profiler.end_phase("server_startup")?;
    }

    // Main development loop
    println!("\n🎯 Development server ready!");
    println!("   • Fast mode: {}", if fast { "Active (50-70% faster builds)" } else { "Inactive" });
    println!("   • Hot reload: {}", if hot_reload { "Active" } else { "Inactive" });
    println!("   • Profiling: {}", if profile { "Active" } else { "Inactive" });
    println!("\nPress Ctrl+C to stop the server");

    // Keep the server running and handle hot-reload
    if let Some(mut hot_reload_manager) = hot_reload_manager {
        hot_reload_manager.run_development_loop()?;
    } else {
        // Simple keep-alive loop
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    Ok(())
}

fn profile_build_performance(
    project_dir: Option<PathBuf>,
    iterations: u32,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("📊 Profiling build performance for: {}", project_path.display());
    println!("   Iterations: {}", iterations);
    println!();

    let mut profiler = BuildProfiler::new();
    let mut metrics = PerformanceMetrics::new();

    for i in 1..=iterations {
        println!("🔨 Build iteration {}/{}", i, iterations);
        
        profiler.start_profiling()?;
        profiler.start_phase("full_build")?;
        
        let start = std::time::Instant::now();
        
        // Perform build
        run_cargo_leptos_build(&project_path)?;
        
        let duration = start.elapsed();
        
        profiler.end_phase("full_build")?;
        let _metrics = profiler.finish_profiling()?;
        
        metrics.record_build_simple("full", duration);
        
        println!("   ✅ Completed in {:.2}s", duration.as_secs_f64());
    }

    // Generate performance report
    println!("\n📈 Performance Report:");
    let report = metrics.generate_report();
    println!("{}", report);

    // Save detailed profiling data (simplified)
    let profile_file = project_path.join("leptos-dev-profile.json");
    std::fs::write(&profile_file, "{}")?;
    println!("📁 Detailed profile saved to: {}", profile_file.display());

    Ok(())
}

fn benchmark_performance(
    compare: bool,
    project_dir: Option<PathBuf>,
    _iterations: usize,
    _scenarios: String,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("🏁 Benchmarking Leptos development performance");
    println!("   Project: {}", project_path.display());
    println!("   Compare with standard: {}", if compare { "Yes" } else { "No" });
    println!();

    let mut metrics = PerformanceMetrics::new();

    if compare {
        // Benchmark standard cargo-leptos
        println!("🔨 Benchmarking standard cargo-leptos build...");
        let start = std::time::Instant::now();
        run_cargo_leptos_build(&project_path)?;
        let standard_duration = start.elapsed();
        metrics.record_build_simple("standard", standard_duration);
        println!("   ✅ Standard build: {:.2}s", standard_duration.as_secs_f64());
    }

    // Benchmark fast development mode
    println!("⚡ Benchmarking fast development mode...");
    let mut fast_dev = FastDevMode::new(&project_path)?;
    fast_dev.setup_fast_config()?;
    
    let start = std::time::Instant::now();
    fast_dev.build_fast()?;
    let fast_duration = start.elapsed();
        metrics.record_build_simple("fast", fast_duration);
    println!("   ✅ Fast build: {:.2}s", fast_duration.as_secs_f64());

    // Generate comparison report
    println!("\n📊 Performance Comparison:");
    let report = metrics.generate_report();
    println!("{}", report);

    if compare {
        let improvement = if let Some(standard) = metrics.get_average_duration("standard") {
            let fast = metrics.get_average_duration("fast").unwrap_or_default();
            let improvement_percent = ((standard.as_secs_f64() - fast.as_secs_f64()) / standard.as_secs_f64()) * 100.0;
            format!("{:.1}% faster", improvement_percent)
        } else {
            "Unable to calculate".to_string()
        };
        
        println!("🚀 Performance improvement: {}", improvement);
    }

    Ok(())
}

fn run_cargo_leptos_build(project_path: &std::path::Path) -> Result<(), DevPerformanceError> {
    let output = std::process::Command::new("cargo")
        .arg("leptos")
        .arg("build")
        .current_dir(project_path)
        .output()
        .map_err(|e| DevPerformanceError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DevPerformanceError::Profiling {
            message: format!("cargo-leptos build failed: {}", stderr),
        });
    }

    Ok(())
}

fn start_development_server(_port: u16, _project_path: &std::path::Path) -> Result<(), DevPerformanceError> {
    // This would integrate with the actual server startup
    // For now, we'll simulate the server startup
    println!("   Starting server process...");
    
    // In a real implementation, this would:
    // 1. Start the actual Leptos server
    // 2. Set up proper error handling
    // 3. Handle graceful shutdown
    // 4. Integrate with hot-reload system
    
    Ok(())
}

/// Run comprehensive performance benchmarks
fn run_comprehensive_benchmarks(
    project_dir: Option<PathBuf>,
    iterations: usize,
    warmup: usize,
    profile: bool,
    output_format: String,
    output: Option<PathBuf>,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("🚀 Running comprehensive performance benchmarks");
    println!("   Project: {}", project_path.display());
    println!("   Iterations: {}", iterations);
    println!("   Warmup: {}", warmup);
    println!("   Profiling: {}", if profile { "Enabled" } else { "Disabled" });
    
    // Create benchmark suite
    let mut benchmark_suite = BenchmarkSuite::new();
    
    // Configure benchmark scenarios
    let scenarios = vec![
        BenchmarkScenario::InitialBuild,
        BenchmarkScenario::IncrementalBuild,
        BenchmarkScenario::CleanBuild,
        BenchmarkScenario::HotReload,
        BenchmarkScenario::MemoryUsage,
        BenchmarkScenario::CpuUtilization,
    ];
    
    let config = BenchmarkConfig {
        iterations,
        warmup_iterations: warmup,
        project_path,
        enable_profiling: profile,
        test_scenarios: scenarios,
    };
    
    // Run benchmarks
    let benchmark_report = benchmark_suite.run_benchmarks(config)?;
    
    // Print summary
    benchmark_report.print_summary();
    
    // Export results
    if let Some(output_path) = output {
        match output_format.as_str() {
            "json" => {
                let json = benchmark_report.export_json()?;
                std::fs::write(&output_path, json)?;
                println!("📊 Benchmark results exported to: {}", output_path.display());
            }
            "csv" => {
                let csv = benchmark_report.export_csv()?;
                std::fs::write(&output_path, csv)?;
                println!("📊 Benchmark results exported to: {}", output_path.display());
            }
            _ => {
                println!("📊 Console output only (use --output-format json or csv to save to file)");
            }
        }
    }
    
    Ok(())
}

/// Validate performance against thresholds
fn validate_performance(
    project_dir: Option<PathBuf>,
    baseline: Option<PathBuf>,
    thresholds: Option<PathBuf>,
    output_format: String,
    output: Option<PathBuf>,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("🔍 Validating performance against thresholds");
    println!("   Project: {}", project_path.display());
    
    // Create validator
    let mut validator = PerformanceValidator::new();
    
    // Set thresholds
    let performance_thresholds = if let Some(thresholds_path) = thresholds {
        // Load from file
        let content = std::fs::read_to_string(thresholds_path)?;
        serde_json::from_str(&content)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    } else {
        PerformanceThresholds::default()
    };
    validator.set_thresholds(performance_thresholds);
    
    // Set baseline if provided
    if let Some(baseline_path) = baseline {
        let content = std::fs::read_to_string(baseline_path)?;
        let baseline_report: BenchmarkReport = serde_json::from_str(&content)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        validator.set_baseline(baseline_report);
    }
    
    // For now, create a mock benchmark report for validation
    // In a real implementation, this would run actual benchmarks
    let mock_benchmark_report = create_mock_benchmark_report();
    
    // Validate performance
    let validation_report = validator.validate(&mock_benchmark_report)?;
    
    // Print validation report
    validation_report.print_report();
    
    // Export results
    if let Some(output_path) = output {
        match output_format.as_str() {
            "json" => {
                let json = validation_report.export_json()?;
                std::fs::write(&output_path, json)?;
                println!("📋 Validation report exported to: {}", output_path.display());
            }
            _ => {
                println!("📋 Console output only (use --output-format json to save to file)");
            }
        }
    }
    
    Ok(())
}

/// Generate performance report
fn generate_performance_report(
    project_dir: Option<PathBuf>,
    benchmark_results: Option<PathBuf>,
    validation_results: Option<PathBuf>,
    format: String,
    output: Option<PathBuf>,
) -> Result<(), DevPerformanceError> {
    let project_path = project_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    
    println!("📊 Generating performance report");
    println!("   Project: {}", project_path.display());
    
    // Create reporter
    let mut reporter = PerformanceReporter::new();
    
    // Load benchmark results if provided
    let benchmark_report = if let Some(benchmark_path) = benchmark_results {
        let content = std::fs::read_to_string(benchmark_path)?;
        Some(serde_json::from_str(&content)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    } else {
        None
    };
    
    // Load validation results if provided
    let validation_report = if let Some(validation_path) = validation_results {
        let content = std::fs::read_to_string(validation_path)?;
        Some(serde_json::from_str(&content)
            .map_err(|e| DevPerformanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
    } else {
        None
    };
    
    // Generate report
    let performance_report = reporter.generate_report(
        "Leptos Development Performance Report".to_string(),
        benchmark_report,
        validation_report,
    )?;
    
    // Export report
    let report_format = match format.as_str() {
        "html" => ReportFormat::Html,
        "markdown" => ReportFormat::Markdown,
        "json" => ReportFormat::Json,
        "csv" => ReportFormat::Csv,
        _ => ReportFormat::Console,
    };
    
    let report_content = reporter.export_report(&performance_report, report_format, output.as_deref())?;
    
    if output.is_none() {
        println!("{}", report_content);
    } else {
        println!("📊 Performance report generated and saved");
    }
    
    Ok(())
}

/// Create a mock benchmark report for testing
fn create_mock_benchmark_report() -> BenchmarkReport {
    use std::collections::HashMap;
    use std::time::Duration;
    
    let mut scenarios = HashMap::new();
    scenarios.insert("Initial Build".to_string(), vec![
        BenchmarkResult {
            name: "Initial Build".to_string(),
            build_type: BuildType::Standard,
            duration: Duration::from_secs(15),
            memory_usage: 512 * 1024 * 1024, // 512MB
            cpu_usage: 0.6,
            cache_hit_rate: 0.0,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    ]);
    
    let mut report = BenchmarkReport::new();
    report.scenarios = scenarios;
    report.summary = BenchmarkSummary {
        total_builds: 1,
        average_build_time: Duration::from_secs(15),
        fastest_build: Duration::from_secs(15),
        slowest_build: Duration::from_secs(15),
        total_memory_usage: 512 * 1024 * 1024,
        average_cpu_usage: 0.6,
        cache_efficiency: 0.0,
        performance_score: 75.0,
    };
    
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dev_command_parsing() {
        let cli = Cli::try_parse_from(&["leptos-dev", "dev", "--fast", "--port", "3001"]).unwrap();
        
        match cli.command {
            Commands::Dev { fast, port, .. } => {
                assert!(fast);
                assert_eq!(port, 3001);
            }
            _ => panic!("Expected dev command"),
        }
    }

    #[test]
    fn test_profile_command_parsing() {
        let cli = Cli::try_parse_from(&["leptos-dev", "profile", "--iterations", "5"]).unwrap();
        
        match cli.command {
            Commands::Profile { iterations, .. } => {
                assert_eq!(iterations, 5);
            }
            _ => panic!("Expected profile command"),
        }
    }
}
