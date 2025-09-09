//! Performance benchmarking tests for the Unified Signal API
//! 
//! This test suite establishes performance baselines and ensures that the
//! unified signal API has parity with existing APIs.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set, Update};
use reactive_graph::owner::Owner;
use reactive_graph::signal::{create_signal, create_rw_signal};
use reactive_graph::computed::create_memo;
use any_spawner::Executor;
use std::time::Instant;

/// Benchmark basic signal operations (get/set/update)
#[test]
fn benchmark_basic_signal_operations() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let unified_signal = signal(owner.clone(), 0);
        let (old_read, old_write) = create_signal(0);
        let old_rw = create_rw_signal(0);
        
        const ITERATIONS: usize = 100_000;
        
        // Benchmark unified signal get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = unified_signal.get();
        }
        let unified_get_time = start.elapsed();
        
        // Benchmark old read signal get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = old_read.get();
        }
        let old_get_time = start.elapsed();
        
        // Benchmark old RwSignal get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = old_rw.get();
        }
        let old_rw_get_time = start.elapsed();
        
        // Benchmark unified signal set operations
        let start = Instant::now();
        for i in 0..ITERATIONS {
            unified_signal.set(i as i32);
        }
        let unified_set_time = start.elapsed();
        
        // Benchmark old write signal set operations
        let start = Instant::now();
        for i in 0..ITERATIONS {
            old_write.set(i as i32);
        }
        let old_set_time = start.elapsed();
        
        // Benchmark old RwSignal set operations
        let start = Instant::now();
        for i in 0..ITERATIONS {
            old_rw.set(i as i32);
        }
        let old_rw_set_time = start.elapsed();
        
        // Benchmark unified signal update operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            unified_signal.update(|val| *val += 1);
        }
        let unified_update_time = start.elapsed();
        
        // Benchmark old write signal update operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            old_write.update(|val| *val += 1);
        }
        let old_update_time = start.elapsed();
        
        // Benchmark old RwSignal update operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            old_rw.update(|val| *val += 1);
        }
        let old_rw_update_time = start.elapsed();
        
        // Performance assertions - unified API should be within 20% of old API
        let get_ratio = unified_get_time.as_nanos() as f64 / old_get_time.as_nanos() as f64;
        let set_ratio = unified_set_time.as_nanos() as f64 / old_set_time.as_nanos() as f64;
        let update_ratio = unified_update_time.as_nanos() as f64 / old_update_time.as_nanos() as f64;
        
        println!("Performance ratios (unified/old):");
        println!("  Get: {:.2}x (unified: {:?}, old: {:?})", get_ratio, unified_get_time, old_get_time);
        println!("  Set: {:.2}x (unified: {:?}, old: {:?})", set_ratio, unified_set_time, old_set_time);
        println!("  Update: {:.2}x (unified: {:?}, old: {:?})", update_ratio, unified_update_time, old_update_time);
        
        // Assert performance is within acceptable bounds
        assert!(get_ratio < 1.2, "Unified signal get operations are too slow: {:.2}x", get_ratio);
        assert!(set_ratio < 1.2, "Unified signal set operations are too slow: {:.2}x", set_ratio);
        assert!(update_ratio < 1.2, "Unified signal update operations are too slow: {:.2}x", update_ratio);
        
        // Also compare with RwSignal
        let get_rw_ratio = unified_get_time.as_nanos() as f64 / old_rw_get_time.as_nanos() as f64;
        let set_rw_ratio = unified_set_time.as_nanos() as f64 / old_rw_set_time.as_nanos() as f64;
        let update_rw_ratio = unified_update_time.as_nanos() as f64 / old_rw_update_time.as_nanos() as f64;
        
        assert!(get_rw_ratio < 1.2, "Unified signal get operations vs RwSignal are too slow: {:.2}x", get_rw_ratio);
        assert!(set_rw_ratio < 1.2, "Unified signal set operations vs RwSignal are too slow: {:.2}x", set_rw_ratio);
        assert!(update_rw_ratio < 1.2, "Unified signal update operations vs RwSignal are too slow: {:.2}x", update_rw_ratio);
    });
}

/// Benchmark derived signal performance
#[test]
fn benchmark_derived_signals() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let base = signal(owner.clone(), 10);
        let derived = base.derive(|b| *b * 2);
        let old_base = create_rw_signal(10);
        let old_derived = create_memo(move |_| old_base.get() * 2);
        
        const ITERATIONS: usize = 50_000;
        
        // Benchmark unified derived signal get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = derived.get();
        }
        let unified_derived_time = start.elapsed();
        
        // Benchmark old memo get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = leptos::unified_signal::Signal::get(&old_derived);
        }
        let old_derived_time = start.elapsed();
        
        // Benchmark changing base and reading derived
        let start = Instant::now();
        for i in 0..ITERATIONS {
            base.set(i as i32);
            let _ = derived.get();
        }
        let unified_change_time = start.elapsed();
        
        let start = Instant::now();
        for i in 0..ITERATIONS {
            old_base.set(i as i32);
            let _ = leptos::unified_signal::Signal::get(&old_derived);
        }
        let old_change_time = start.elapsed();
        
        let derived_ratio = unified_derived_time.as_nanos() as f64 / old_derived_time.as_nanos() as f64;
        let change_ratio = unified_change_time.as_nanos() as f64 / old_change_time.as_nanos() as f64;
        
        println!("Derived signal performance ratios (unified/old):");
        println!("  Get: {:.2}x (unified: {:?}, old: {:?})", derived_ratio, unified_derived_time, old_derived_time);
        println!("  Change+Get: {:.2}x (unified: {:?}, old: {:?})", change_ratio, unified_change_time, old_change_time);
        
        assert!(derived_ratio < 1.2, "Unified derived signal get operations are too slow: {:.2}x", derived_ratio);
        assert!(change_ratio < 1.2, "Unified derived signal change+get operations are too slow: {:.2}x", change_ratio);
    });
}

/// Benchmark computed signal performance
#[test]
fn benchmark_computed_signals() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let a = signal(owner.clone(), 1);
        let b = signal(owner.clone(), 2);
        let c = signal(owner.clone(), 3);
        
        let a_clone = a.clone();
        let b_clone = b.clone();
        let c_clone = c.clone();
        let computed = signal_module::computed(owner.clone(), move || a_clone.get() + b_clone.get() + c_clone.get());
        
        let old_a = create_rw_signal(1);
        let old_b = create_rw_signal(2);
        let old_c = create_rw_signal(3);
        let old_computed = create_memo(move |_| old_a.get() + old_b.get() + old_c.get());
        
        const ITERATIONS: usize = 25_000;
        
        // Benchmark unified computed signal get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = computed.get();
        }
        let unified_computed_time = start.elapsed();
        
        // Benchmark old memo get operations
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = leptos::unified_signal::Signal::get(&old_computed);
        }
        let old_computed_time = start.elapsed();
        
        // Benchmark changing dependencies and reading computed
        let start = Instant::now();
        for i in 0..ITERATIONS {
            a.set(i as i32);
            b.set((i * 2) as i32);
            c.set((i * 3) as i32);
            let _ = computed.get();
        }
        let unified_change_time = start.elapsed();
        
        let start = Instant::now();
        for i in 0..ITERATIONS {
            old_a.set(i as i32);
            old_b.set((i * 2) as i32);
            old_c.set((i * 3) as i32);
            let _ = leptos::unified_signal::Signal::get(&old_computed);
        }
        let old_change_time = start.elapsed();
        
        let computed_ratio = unified_computed_time.as_nanos() as f64 / old_computed_time.as_nanos() as f64;
        let change_ratio = unified_change_time.as_nanos() as f64 / old_change_time.as_nanos() as f64;
        
        println!("Computed signal performance ratios (unified/old):");
        println!("  Get: {:.2}x (unified: {:?}, old: {:?})", computed_ratio, unified_computed_time, old_computed_time);
        println!("  Change+Get: {:.2}x (unified: {:?}, old: {:?})", change_ratio, unified_change_time, old_change_time);
        
        assert!(computed_ratio < 1.2, "Unified computed signal get operations are too slow: {:.2}x", computed_ratio);
        assert!(change_ratio < 1.2, "Unified computed signal change+get operations are too slow: {:.2}x", change_ratio);
    });
}

/// Benchmark signal creation performance
#[test]
fn benchmark_signal_creation() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 10_000;
        
        // Benchmark unified signal creation
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let _ = signal(owner.clone(), i);
        }
        let unified_creation_time = start.elapsed();
        
        // Benchmark old signal creation
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let _ = create_signal(i);
        }
        let old_creation_time = start.elapsed();
        
        // Benchmark old RwSignal creation
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let _ = create_rw_signal(i);
        }
        let old_rw_creation_time = start.elapsed();
        
        let creation_ratio = unified_creation_time.as_nanos() as f64 / old_creation_time.as_nanos() as f64;
        let rw_creation_ratio = unified_creation_time.as_nanos() as f64 / old_rw_creation_time.as_nanos() as f64;
        
        println!("Signal creation performance ratios (unified/old):");
        println!("  vs create_signal: {:.2}x (unified: {:?}, old: {:?})", creation_ratio, unified_creation_time, old_creation_time);
        println!("  vs create_rw_signal: {:.2}x (unified: {:?}, old: {:?})", rw_creation_ratio, unified_creation_time, old_rw_creation_time);
        
        assert!(creation_ratio < 1.5, "Unified signal creation is too slow: {:.2}x", creation_ratio);
        assert!(rw_creation_ratio < 1.5, "Unified signal creation vs RwSignal is too slow: {:.2}x", rw_creation_ratio);
    });
}

/// Benchmark memory usage patterns
#[test]
fn benchmark_memory_usage() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const SIGNAL_COUNT: usize = 1_000;
        
        // Create many unified signals
        let start = Instant::now();
        let mut unified_signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            unified_signals.push(signal(owner.clone(), i));
        }
        let unified_creation_time = start.elapsed();
        
        // Create many old signals
        let start = Instant::now();
        let mut old_signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            old_signals.push(create_signal(i));
        }
        let old_creation_time = start.elapsed();
        
        // Create many old RwSignals
        let start = Instant::now();
        let mut old_rw_signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            old_rw_signals.push(create_rw_signal(i));
        }
        let old_rw_creation_time = start.elapsed();
        
        // Test that all signals work correctly
        for (i, signal) in unified_signals.iter().enumerate() {
            assert_eq!(signal.get(), i);
        }
        
        for (i, (read, _)) in old_signals.iter().enumerate() {
            assert_eq!(read.get(), i);
        }
        
        for (i, signal) in old_rw_signals.iter().enumerate() {
            assert_eq!(signal.get(), i);
        }
        
        let creation_ratio = unified_creation_time.as_nanos() as f64 / old_creation_time.as_nanos() as f64;
        let rw_creation_ratio = unified_creation_time.as_nanos() as f64 / old_rw_creation_time.as_nanos() as f64;
        
        println!("Bulk signal creation performance ratios (unified/old):");
        println!("  vs create_signal: {:.2}x (unified: {:?}, old: {:?})", creation_ratio, unified_creation_time, old_creation_time);
        println!("  vs create_rw_signal: {:.2}x (unified: {:?}, old: {:?})", rw_creation_ratio, unified_creation_time, old_rw_creation_time);
        
        assert!(creation_ratio < 2.0, "Unified bulk signal creation is too slow: {:.2}x", creation_ratio);
        assert!(rw_creation_ratio < 2.0, "Unified bulk signal creation vs RwSignal is too slow: {:.2}x", rw_creation_ratio);
    });
}

/// Benchmark signal splitting performance
#[test]
fn benchmark_signal_splitting() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 10_000;
        
        // Benchmark unified signal splitting
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let signal = signal(owner.clone(), i);
            let (read, write) = signal.split();
            let _ = read.get();
            write.set(i + 1);
        }
        let unified_splitting_time = start.elapsed();
        
        // Benchmark old signal creation (which is already split)
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let (read, write) = create_signal(i);
            let _ = read.get();
            write.set(i + 1);
        }
        let old_splitting_time = start.elapsed();
        
        let splitting_ratio = unified_splitting_time.as_nanos() as f64 / old_splitting_time.as_nanos() as f64;
        
        println!("Signal splitting performance ratio (unified/old):");
        println!("  {:.2}x (unified: {:?}, old: {:?})", splitting_ratio, unified_splitting_time, old_splitting_time);
        
        assert!(splitting_ratio < 2.0, "Unified signal splitting is too slow: {:.2}x", splitting_ratio);
    });
}

/// Benchmark complex signal graph performance
#[test]
fn benchmark_complex_signal_graph() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const NODES: usize = 100;
        const ITERATIONS: usize = 1_000;
        
        // Create a complex graph with unified signals
        let mut unified_nodes = Vec::new();
        for i in 0..NODES {
            unified_nodes.push(signal(owner.clone(), i as i32));
        }
        
        // Create derived signals that depend on multiple nodes
        let mut unified_derived = Vec::new();
        for i in 0..NODES / 2 {
            let node1 = unified_nodes[i].clone();
            let node2 = unified_nodes[i + NODES / 2].clone();
            unified_derived.push(node1.derive(move |n1| {
                let n2_val = node2.get();
                *n1 + n2_val
            }));
        }
        
        // Create a complex graph with old signals
        let mut old_nodes = Vec::new();
        for i in 0..NODES {
            old_nodes.push(create_rw_signal(i as i32));
        }
        
        let mut old_derived = Vec::new();
        for i in 0..NODES / 2 {
            let node1 = old_nodes[i].clone();
            let node2 = old_nodes[i + NODES / 2].clone();
            old_derived.push(create_memo(move |_| {
                let n1_val = node1.get();
                let n2_val = node2.get();
                n1_val + n2_val
            }));
        }
        
        // Benchmark updating the graph
        let start = Instant::now();
        for iter in 0..ITERATIONS {
            for (i, node) in unified_nodes.iter().enumerate() {
                node.set((iter * NODES + i) as i32);
            }
            // Read some derived values
            for derived in &unified_derived {
                let _ = leptos::unified_signal::Signal::get(derived);
            }
        }
        let unified_graph_time = start.elapsed();
        
        let start = Instant::now();
        for iter in 0..ITERATIONS {
            for (i, node) in old_nodes.iter().enumerate() {
                node.set((iter * NODES + i) as i32);
            }
            // Read some derived values
            for derived in &old_derived {
                let _ = leptos::unified_signal::Signal::get(derived);
            }
        }
        let old_graph_time = start.elapsed();
        
        let graph_ratio = unified_graph_time.as_nanos() as f64 / old_graph_time.as_nanos() as f64;
        
        println!("Complex signal graph performance ratio (unified/old):");
        println!("  {:.2}x (unified: {:?}, old: {:?})", graph_ratio, unified_graph_time, old_graph_time);
        
        assert!(graph_ratio < 1.5, "Unified complex signal graph is too slow: {:.2}x", graph_ratio);
    });
}
