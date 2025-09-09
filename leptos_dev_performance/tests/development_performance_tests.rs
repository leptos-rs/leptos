//! Development Performance Tests - TDD Approach
//!
//! These tests define the performance targets identified in the roadmap:
//! - Eliminate 30+ second compilation times 
//! - Fast development mode for rapid iteration
//! - Hot-reload reliability improvements
//! - Build-time profiling and monitoring

use leptos_dev_performance::{
    BuildProfiler, FastDevMode, HotReloadManager, IncrementalCompiler,
    PerformanceMetrics, PerformanceTargets
};
use std::time::Duration;
use tempfile::TempDir;

/// Test that development builds complete within acceptable time limits
/// TARGET: <5 seconds for incremental changes, <15 seconds for full rebuilds
#[test]
fn test_development_build_performance_targets() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("test-project");
    
    // Create a realistic Leptos project structure
    create_test_leptos_project(&project_path);
    
    let profiler = BuildProfiler::new();
    let mut fast_dev = FastDevMode::new(&project_path).expect("Failed to initialize fast dev mode");
    
    // TEST 1: Initial build should complete in reasonable time
    let initial_build_start = std::time::Instant::now();
    let initial_result = fast_dev.build_project()
        .expect("Initial build should succeed");
    let initial_build_time = initial_build_start.elapsed();
    
    // FAILING TEST: Current baseline likely exceeds this
    assert!(
        initial_build_time < Duration::from_secs(15),
        "Initial development build took {:?}, should be <15s. This indicates the P0 performance crisis.",
        initial_build_time
    );
    
    // TEST 2: Incremental builds should be very fast
    // Simulate small component change
    modify_component_file(&project_path, "src/components/app.rs", "// Small change");
    
    let incremental_build_start = std::time::Instant::now();
    let incremental_result = fast_dev.incremental_build()
        .expect("Incremental build should succeed");
    let incremental_build_time = incremental_build_start.elapsed();
    
    // FAILING TEST: This is the critical target
    assert!(
        incremental_build_time < Duration::from_secs(5),
        "Incremental build took {:?}, should be <5s. This addresses the development performance crisis.",
        incremental_build_time
    );
    
    // TEST 3: Build time should be predictable and not degrade
    let mut build_times = Vec::new();
    for i in 0..5 {
        modify_component_file(&project_path, "src/components/app.rs", &format!("// Change {}", i));
        
        let start = std::time::Instant::now();
        fast_dev.incremental_build().expect("Build should succeed");
        build_times.push(start.elapsed());
    }
    
    // Build times should not show significant regression
    let avg_time = build_times.iter().sum::<Duration>() / build_times.len() as u32;
    let max_time = build_times.iter().max().unwrap();
    
    assert!(
        *max_time < avg_time * 2,
        "Build time regression detected: max {:?}, avg {:?}",
        max_time, avg_time
    );
    
    println!("✅ Development build performance test completed");
}

/// Test hot-reload reliability and speed
/// TARGET: <2 second change propagation, 95% reliability
#[test]
fn test_hot_reload_reliability() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("hot-reload-project");
    
    create_test_leptos_project(&project_path);
    
    let mut hot_reload = HotReloadManager::new(&project_path)
        .expect("Failed to initialize hot reload");
    
    // Start hot-reload server
    hot_reload.start().expect("Failed to start hot reload");
    
    // TEST 1: File change detection speed
    let change_start = std::time::Instant::now();
    modify_component_file(&project_path, "src/components/counter.rs", "// Hot reload test");
    
    // Wait for change detection
    let change_detected = hot_reload.wait_for_change(Duration::from_secs(3))
        .expect("Change should be detected within 3 seconds");
    let detection_time = change_start.elapsed();
    
    // FAILING TEST: Current implementation may be slow or unreliable
    assert!(
        detection_time < Duration::from_secs(2),
        "Hot reload change detection took {:?}, should be <2s",
        detection_time
    );
    
    // TEST 2: Reload success rate
    let mut successful_reloads = 0;
    let total_attempts = 10;
    
    for i in 0..total_attempts {
        modify_component_file(
            &project_path, 
            "src/components/counter.rs", 
            &format!("// Reload test {}", i)
        );
        
        if hot_reload.wait_for_reload(Duration::from_secs(5)).is_ok() {
            successful_reloads += 1;
        }
    }
    
    let success_rate = (successful_reloads as f64) / (total_attempts as f64);
    
    // FAILING TEST: Current hot-reload may be unreliable
    assert!(
        success_rate >= 0.95,
        "Hot reload success rate {:.1}%, should be ≥95%",
        success_rate * 100.0
    );
    
    println!("✅ Hot reload reliability test completed");
}

/// Test build profiling and monitoring capabilities
/// TARGET: Detailed insights into build bottlenecks
#[test] 
fn test_build_profiling_insights() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("profiling-project");
    
    create_test_leptos_project(&project_path);
    
    let mut profiler = BuildProfiler::new();
    profiler.enable_detailed_profiling();
    
    // Perform build with profiling
    let mut fast_dev = FastDevMode::new(&project_path)
        .expect("Failed to initialize fast dev mode");
    
    profiler.start_profiling();
    let result = fast_dev.build_project().expect("Build should succeed");
    let metrics = profiler.finish_profiling().expect("Profiling should succeed");
    
    // TEST 1: Profiler should identify build phases
    assert!(metrics.phases.len() >= 5, "Should identify major build phases");
    
    // Expected phases: dependency resolution, macro expansion, compilation, linking, asset processing
    let expected_phases = ["dependency_resolution", "macro_expansion", "compilation", "linking"];
    for phase in &expected_phases {
        assert!(
            metrics.phases.contains_key(*phase),
            "Should profile '{}' phase",
            phase
        );
    }
    
    // TEST 2: Should identify performance bottlenecks
    let bottleneck = metrics.bottlenecks.first();
    assert!(bottleneck.is_some(), "Should identify primary bottleneck");
    
    let bottleneck = bottleneck.unwrap();
    assert!(
        bottleneck.duration > Duration::from_millis(100),
        "Bottleneck should be significant enough to matter"
    );
    
    // TEST 3: Should provide actionable recommendations
    let recommendations = &metrics.recommendations;
    assert!(!recommendations.is_empty(), "Should provide optimization recommendations");
    
    // Verify recommendations are actionable
    for rec in recommendations {
        assert!(!rec.description.is_empty(), "Recommendation should have description");
        assert!(!rec.action.is_empty(), "Recommendation should have actionable steps");
        assert!(rec.impact_estimate.is_some(), "Should estimate impact");
    }
    
    println!("✅ Build profiling insights test completed");
    println!("Primary bottleneck: {} ({:?})", bottleneck.phase, bottleneck.duration);
    println!("Optimization recommendations: {}", recommendations.len());
}

/// Test incremental compilation system
/// TARGET: Only recompile changed modules and dependencies
#[test]
fn test_incremental_compilation_correctness() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("incremental-project");
    
    create_test_leptos_project(&project_path);
    
    let mut incremental = IncrementalCompiler::new(&project_path)
        .expect("Failed to initialize incremental compiler");
    
    // Initial full compilation
    let initial_result = incremental.full_compile()
        .expect("Initial compilation should succeed");
    
    assert!(initial_result.modules_compiled > 0, "Should compile modules initially");
    let initial_modules = initial_result.modules_compiled;
    
    // TEST 1: No changes should trigger no recompilation
    let no_change_result = incremental.incremental_compile()
        .expect("No-change compilation should succeed");
    
    assert_eq!(
        no_change_result.modules_compiled, 0,
        "No changes should not trigger recompilation"
    );
    
    // TEST 2: Single file change should only recompile affected modules
    modify_component_file(&project_path, "src/components/counter.rs", "// Incremental test");
    
    let single_change_result = incremental.incremental_compile()
        .expect("Single change compilation should succeed");
    
    // FAILING TEST: Current implementation may over-compile
    assert!(
        single_change_result.modules_compiled < initial_modules / 2,
        "Single file change compiled {} modules, should be much less than initial {}",
        single_change_result.modules_compiled, initial_modules
    );
    
    // TEST 3: Dependency tracking accuracy
    // Change a dependency and verify dependents are recompiled
    modify_component_file(&project_path, "src/lib.rs", "// Library change");
    
    let dep_change_result = incremental.incremental_compile()
        .expect("Dependency change compilation should succeed");
    
    // Library changes should trigger more recompilation but still be selective
    assert!(
        dep_change_result.modules_compiled > single_change_result.modules_compiled,
        "Library change should trigger more recompilation than component change"
    );
    
    assert!(
        dep_change_result.modules_compiled < initial_modules,
        "Even library changes shouldn't require full recompilation"
    );
    
    println!("✅ Incremental compilation correctness test completed");
}

/// Performance regression test to prevent future slowdowns
/// TARGET: Maintain performance improvements over time
#[test]
fn test_performance_regression_prevention() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("regression-project");
    
    create_test_leptos_project(&project_path);
    
    let mut fast_dev = FastDevMode::new(&project_path)
        .expect("Failed to initialize fast dev mode");
    
    // Establish baseline performance
    let baseline_times = measure_build_times(&mut fast_dev, 5);
    let baseline_avg = baseline_times.iter().sum::<Duration>() / baseline_times.len() as u32;
    
    // Simulate various project sizes and complexities
    let test_scenarios = [
        ("small_project", 5),   // 5 components
        ("medium_project", 20), // 20 components  
        ("large_project", 50),  // 50 components
    ];
    
    for (scenario_name, component_count) in test_scenarios {
        create_project_with_components(&project_path, component_count);
        
        let scenario_times = measure_build_times(&mut fast_dev, 3);
        let scenario_avg = scenario_times.iter().sum::<Duration>() / scenario_times.len() as u32;
        
        // Performance should scale reasonably with project size
        let scale_factor = component_count as f64 / 5.0; // Relative to small project
        let expected_max_time = baseline_avg.mul_f64(scale_factor * 1.5); // Allow 50% overhead
        
        assert!(
            scenario_avg < expected_max_time,
            "Performance regression in {}: {:?} > expected {:?}",
            scenario_name, scenario_avg, expected_max_time
        );
        
        println!("✅ {} performance: {:?} (baseline: {:?})", 
                scenario_name, scenario_avg, baseline_avg);
    }
    
    println!("✅ Performance regression prevention test completed");
}

// Helper functions for test setup and execution

fn create_test_leptos_project(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("Failed to create project directory");
    std::fs::create_dir_all(path.join("src/components")).expect("Failed to create components dir");
    
    // Create Cargo.toml
    std::fs::write(
        path.join("Cargo.toml"),
        r#"[package]
name = "test-leptos-app"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { workspace = true }
"#,
    ).expect("Failed to create Cargo.toml");
    
    // Create lib.rs
    std::fs::write(
        path.join("src/lib.rs"),
        r#"pub mod components;
pub use components::*;
"#,
    ).expect("Failed to create lib.rs");
    
    // Create components/mod.rs
    std::fs::write(
        path.join("src/components/mod.rs"),
        r#"pub mod app;
pub mod counter;
pub use app::*;
pub use counter::*;
"#,
    ).expect("Failed to create components/mod.rs");
    
    // Create components/app.rs
    std::fs::write(
        path.join("src/components/app.rs"),
        r#"use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app">
            <h1>"Test Leptos App"</h1>
            <Counter initial_value=0/>
        </div>
    }
}
"#,
    ).expect("Failed to create app.rs");
    
    // Create components/counter.rs
    std::fs::write(
        path.join("src/components/counter.rs"),
        r#"use leptos::*;

#[component]
pub fn Counter(initial_value: i32) -> impl IntoView {
    let (count, set_count) = create_signal(initial_value);
    
    view! {
        <div class="counter">
            <p>"Count: " {count}</p>
            <button on:click=move |_| set_count.update(|c| *c += 1)>
                "+"
            </button>
            <button on:click=move |_| set_count.update(|c| *c -= 1)>
                "-"
            </button>
        </div>
    }
}
"#,
    ).expect("Failed to create counter.rs");
}

fn modify_component_file(project_path: &std::path::Path, file_path: &str, addition: &str) {
    let full_path = project_path.join(file_path);
    let mut content = std::fs::read_to_string(&full_path)
        .expect("Failed to read file for modification");
    content.push_str("\n");
    content.push_str(addition);
    std::fs::write(&full_path, content).expect("Failed to write modified file");
}

fn measure_build_times(fast_dev: &mut FastDevMode, iterations: usize) -> Vec<Duration> {
    let mut times = Vec::new();
    
    for i in 0..iterations {
        // Make a small change to trigger rebuild
        let change = format!("// Measurement iteration {}", i);
        
        let start = std::time::Instant::now();
        fast_dev.incremental_build().expect("Build should succeed");
        times.push(start.elapsed());
    }
    
    times
}

fn create_project_with_components(project_path: &std::path::Path, component_count: usize) {
    for i in 0..component_count {
        let component_name = format!("Component{}", i);
        let file_content = format!(
            r#"use leptos::*;

#[component]
pub fn {}() -> impl IntoView {{
    let (state, set_state) = create_signal(0);
    
    view! {{
        <div class="component-{}">
            <p>"Component {}: " {{state}}</p>
            <button on:click=move |_| set_state.update(|s| *s += 1)>
                "Update"
            </button>
        </div>
    }}
}}
"#,
            component_name, i, i
        );
        
        std::fs::write(
            project_path.join(format!("src/components/{}.rs", component_name.to_lowercase())),
            file_content,
        ).expect("Failed to create component file");
    }
}