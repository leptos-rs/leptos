use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use leptos_performance_optimizations::{
    OptimizedSignal, UpdateBatch, EffectScheduler, EffectPriority,
    SubscriberStorage, AnySubscriber, SubscriberId
};

fn bench_signal_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_updates");
    
    // Test different numbers of subscribers
    for subscriber_count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*subscriber_count as u64));
        
        group.bench_with_input(
            BenchmarkId::new("individual_updates", subscriber_count),
            subscriber_count,
            |b, &count| {
                let signal = OptimizedSignal::new(0i32);
                
                // Create subscribers (simulated)
                for _ in 0..count {
                    // In real implementation, this would create actual effects
                }
                
                b.iter(|| {
                    for i in 0..100 {
                        signal.set(black_box(i));
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("batched_updates", subscriber_count),
            subscriber_count,
            |b, &count| {
                let signal = OptimizedSignal::new(0i32);
                
                // Create subscribers
                for _ in 0..count {
                    // In real implementation, create effects
                }
                
                b.iter(|| {
                    UpdateBatch::batch_updates(|| {
                        for i in 0..100 {
                            signal.set(black_box(i));
                        }
                    });
                });
            },
        );
    }
    
    group.finish();
}

fn bench_subscriber_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("subscriber_storage");
    
    // Test adding subscribers to storage
    group.bench_function("add_subscribers_inline", |b| {
        b.iter(|| {
            let mut storage = SubscriberStorage::new();
            for i in 0..3 {
                storage.add_subscriber(AnySubscriber {
                    id: SubscriberId(i),
                });
            }
            black_box(storage);
        });
    });

    group.bench_function("add_subscribers_heap", |b| {
        b.iter(|| {
            let mut storage = SubscriberStorage::new();
            for i in 0..10 {
                storage.add_subscriber(AnySubscriber {
                    id: SubscriberId(i),
                });
            }
            black_box(storage);
        });
    });

    // Test notification performance
    group.bench_function("notify_inline_subscribers", |b| {
        let mut storage = SubscriberStorage::new();
        for i in 0..3 {
            storage.add_subscriber(AnySubscriber {
                id: SubscriberId(i),
            });
        }

        b.iter(|| {
            storage.notify_subscribers();
        });
    });

    group.bench_function("notify_heap_subscribers", |b| {
        let mut storage = SubscriberStorage::new();
        for i in 0..100 {
            storage.add_subscriber(AnySubscriber {
                id: SubscriberId(i),
            });
        }

        b.iter(|| {
            storage.notify_subscribers();
        });
    });

    group.finish();
}

fn bench_batch_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_performance");
    
    group.bench_function("single_update", |b| {
        let signal = OptimizedSignal::new(0i32);
        
        b.iter(|| {
            signal.set(black_box(42));
        });
    });

    group.bench_function("batched_multiple_signals", |b| {
        let signals: Vec<_> = (0..10)
            .map(|i| OptimizedSignal::new(i))
            .collect();
        
        b.iter(|| {
            UpdateBatch::batch_updates(|| {
                for (i, signal) in signals.iter().enumerate() {
                    signal.set(black_box(i as i32 + 100));
                }
            });
        });
    });

    group.bench_function("nested_batches", |b| {
        let signal1 = OptimizedSignal::new(0i32);
        let signal2 = OptimizedSignal::new(0i32);
        
        b.iter(|| {
            UpdateBatch::batch_updates(|| {
                signal1.set(1);
                UpdateBatch::batch_updates(|| {
                    signal2.set(2);
                });
                signal1.set(3);
            });
        });
    });

    group.finish();
}

fn bench_effect_scheduling(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_scheduling");
    
    // Test effect scheduling performance
    for priority in [EffectPriority::Immediate, EffectPriority::Normal, EffectPriority::Low].iter() {
        group.bench_with_input(
            BenchmarkId::new("schedule_effects", format!("{:?}", priority)),
            priority,
            |b, &priority| {
                b.iter(|| {
                    for _ in 0..100 {
                        // In real implementation, would create and schedule actual effects
                        black_box(priority);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");
    
    // Test allocation patterns
    group.bench_function("vec_allocation", |b| {
        b.iter(|| {
            let mut vec = Vec::new();
            for i in 0..100 {
                vec.push(black_box(i));
            }
            black_box(vec);
        });
    });

    group.bench_function("smallvec_allocation", |b| {
        b.iter(|| {
            let mut vec = smallvec::SmallVec::<[i32; 8]>::new();
            for i in 0..100 {
                vec.push(black_box(i));
            }
            black_box(vec);
        });
    });

    // Test cloning overhead
    group.bench_function("clone_large_vec", |b| {
        let large_vec: Vec<i32> = (0..1000).collect();
        
        b.iter(|| {
            let cloned = black_box(large_vec.clone());
            black_box(cloned);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_signal_updates,
    bench_subscriber_storage,
    bench_batch_performance,
    bench_effect_scheduling,
    bench_memory_patterns
);

criterion_main!(benches);