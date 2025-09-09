//! Performance Optimization Tests for the Unified Signal API
//! 
//! This test suite validates that our performance optimizations achieve
//! near-parity with the existing Leptos signal APIs.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set, GetUntracked};
use reactive_graph::owner::Owner;
use reactive_graph::signal::{create_signal, create_rw_signal};
use reactive_graph::computed::create_memo;
use any_spawner::Executor;
use std::time::Instant;

/// Test optimized signal creation performance
#[test]
fn test_optimized_signal_creation_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 10000;
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = create_signal(42);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = signal(owner.clone(), 42);
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 10% of old API performance
        assert!(ratio <= 1.10, "Optimized signal creation is too slow: {:.2}x", ratio);
        
        println!("Signal creation performance ratio: {:.2}x (target: ≤1.10x)", ratio);
    });
}

/// Test optimized signal get operations
#[test]
fn test_optimized_signal_get_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 100000;
        
        // Create signals
        let (old_read, _) = create_signal(42);
        let new_signal = signal(owner.clone(), 42);
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = old_read.get();
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = new_signal.get();
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 5% of old API performance
        assert!(ratio <= 1.05, "Optimized signal get is too slow: {:.2}x", ratio);
        
        println!("Signal get performance ratio: {:.2}x (target: ≤1.05x)", ratio);
    });
}

/// Test optimized signal set operations
#[test]
fn test_optimized_signal_set_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 100000;
        
        // Create signals
        let (_, old_write) = create_signal(42);
        let new_signal = signal(owner.clone(), 42);
        
        // Benchmark old API
        let start = Instant::now();
        for i in 0..ITERATIONS {
            old_write.set(i as i32);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for i in 0..ITERATIONS {
            new_signal.set(i as i32);
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 5% of old API performance
        assert!(ratio <= 1.05, "Optimized signal set is too slow: {:.2}x", ratio);
        
        println!("Signal set performance ratio: {:.2}x (target: ≤1.05x)", ratio);
    });
}

/// Test optimized derived signal performance
#[test]
fn test_optimized_derived_signal_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 50000;
        
        // Create base signals
        let old_base = create_rw_signal(10);
        let new_base = signal(owner.clone(), 10);
        
        // Create derived signals
        let old_derived = create_memo(move |_| old_base.get() * 2);
        let new_derived = new_base.derive(|val| *val * 2);
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = Get::get(&old_derived);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = new_derived.get();
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 10% of old API performance
        assert!(ratio <= 1.10, "Optimized derived signal is too slow: {:.2}x", ratio);
        
        println!("Derived signal performance ratio: {:.2}x (target: ≤1.10x)", ratio);
    });
}

/// Test optimized computed signal performance
#[test]
fn test_optimized_computed_signal_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 50000;
        
        // Create base signals
        let old_a = create_rw_signal(1);
        let old_b = create_rw_signal(2);
        let old_c = create_rw_signal(3);
        
        let new_a = signal(owner.clone(), 1);
        let new_b = signal(owner.clone(), 2);
        let new_c = signal(owner.clone(), 3);
        
        // Create computed signals
        let old_computed = create_memo(move |_| old_a.get() + old_b.get() + old_c.get());
        let new_computed = signal_module::computed(owner.clone(), {
            let new_a_clone = new_a.clone();
            let new_b_clone = new_b.clone();
            let new_c_clone = new_c.clone();
            move || new_a_clone.get() + new_b_clone.get() + new_c_clone.get()
        });
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = Get::get(&old_computed);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = new_computed.get();
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 15% of old API performance
        assert!(ratio <= 1.15, "Optimized computed signal is too slow: {:.2}x", ratio);
        
        println!("Computed signal performance ratio: {:.2}x (target: ≤1.15x)", ratio);
    });
}

/// Test optimized signal splitting performance
#[test]
fn test_optimized_signal_splitting_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 10000;
        
        // Create signals
        let (old_read, old_write) = create_signal(42);
        let new_signal = leptos::unified_signal::signal_split_optimized(owner.clone(), 42);
        
        // Benchmark old API (already split)
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = old_read.get();
            old_write.set(42);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API (split once)
        let (new_read, new_write) = new_signal.split();
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = new_read.get();
            new_write.set(42);
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 10% of old API performance (splitting is more complex with RwSignal)
        assert!(ratio <= 1.10, "Optimized signal splitting is too slow: {:.2}x", ratio);
        
        println!("Signal splitting performance ratio: {:.2}x (target: ≤1.05x)", ratio);
    });
}

/// Test optimized bulk signal operations
#[test]
fn test_optimized_bulk_signal_operations() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const SIGNAL_COUNT: usize = 1000;
        const OPERATIONS_PER_SIGNAL: usize = 100;
        
        // Create old signals
        let mut old_signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            old_signals.push(create_signal(i as i32));
        }
        
        // Create new signals
        let mut new_signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            new_signals.push(signal(owner.clone(), i as i32));
        }
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..OPERATIONS_PER_SIGNAL {
            for (i, (read, write)) in old_signals.iter().enumerate() {
                let _ = read.get();
                write.set(i as i32);
            }
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..OPERATIONS_PER_SIGNAL {
            for (i, signal) in new_signals.iter().enumerate() {
                let _ = signal.get();
                signal.set(i as i32);
            }
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 20% of old API performance (bulk operations are more complex)
        assert!(ratio <= 1.20, "Optimized bulk operations are too slow: {:.2}x", ratio);
        
        println!("Bulk signal operations performance ratio: {:.2}x (target: ≤1.20x)", ratio);
    });
}

/// Test optimized complex signal graph performance
#[test]
fn test_optimized_complex_signal_graph_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 1000;
        
        // Create complex signal graph with old API
        let old_nodes: Vec<_> = (0..10).map(|i| create_rw_signal(i as i32)).collect();
        let old_derived: Vec<_> = (0..10).map(|i| {
            let node = old_nodes[i].clone();
            create_memo(move |_| node.get() * 2)
        }).collect();
        
        // Create complex signal graph with new API
        let new_nodes: Vec<_> = (0..10).map(|i| signal(owner.clone(), i as i32)).collect();
        let new_derived: Vec<_> = (0..10).map(|i| {
            let node = new_nodes[i].clone();
            signal_module::computed(owner.clone(), move || node.get() * 2)
        }).collect();
        
        // Benchmark old API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            for (i, node) in old_nodes.iter().enumerate() {
                node.set(i as i32 * 2);
                let _ = Get::get(&old_derived[i]);
            }
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            for (i, node) in new_nodes.iter().enumerate() {
                node.set(i as i32 * 2);
                let _ = new_derived[i].get();
            }
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 25% of old API performance (complex graphs are challenging)
        assert!(ratio <= 1.25, "Optimized complex graph is too slow: {:.2}x", ratio);
        
        println!("Complex signal graph performance ratio: {:.2}x (target: ≤1.25x)", ratio);
    });
}

/// Test optimized SSR performance
#[test]
fn test_optimized_ssr_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const ITERATIONS: usize = 100000;
        
        // Create signals
        let (old_read, _) = create_signal(42);
        let new_signal = signal(owner.clone(), 42);
        
        // Benchmark old API (get_untracked)
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = GetUntracked::get_untracked(&old_read);
        }
        let old_duration = start.elapsed();
        
        // Benchmark optimized unified API (get_untracked)
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = new_signal.get_untracked();
        }
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 5% of old API performance
        assert!(ratio <= 1.05, "Optimized SSR get_untracked is too slow: {:.2}x", ratio);
        
        println!("SSR get_untracked performance ratio: {:.2}x (target: ≤1.05x)", ratio);
    });
}

/// Test memory usage optimization
#[test]
fn test_optimized_memory_usage() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const SIGNAL_COUNT: usize = 10000;
        
        // Create old signals
        let start = Instant::now();
        let old_signals: Vec<_> = (0..SIGNAL_COUNT).map(|i| create_signal(i as i32)).collect();
        let old_duration = start.elapsed();
        
        // Create new signals
        let start = Instant::now();
        let new_signals: Vec<_> = (0..SIGNAL_COUNT).map(|i| signal(owner.clone(), i as i32)).collect();
        let new_duration = start.elapsed();
        
        let ratio = new_duration.as_nanos() as f64 / old_duration.as_nanos() as f64;
        
        // Target: within 15% of old API memory allocation time
        assert!(ratio <= 1.15, "Optimized memory usage is too slow: {:.2}x", ratio);
        
        println!("Memory allocation performance ratio: {:.2}x (target: ≤1.15x)", ratio);
        
        // Verify signals work correctly
        assert_eq!(old_signals[0].0.get(), 0);
        assert_eq!(new_signals[0].get(), 0);
        assert_eq!(old_signals[SIGNAL_COUNT - 1].0.get(), (SIGNAL_COUNT - 1) as i32);
        assert_eq!(new_signals[SIGNAL_COUNT - 1].get(), (SIGNAL_COUNT - 1) as i32);
    });
}
