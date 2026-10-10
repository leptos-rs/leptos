//! Micro-benchmarks for the `StoreField` path-based trigger lookups.
//!
//! These exercise the code path targeted by issue #4057: the ancestor walk in
//! `triggers_for_path` pops segments one at a time and used to clone the
//! remaining `StorePath` at every step.
//!
//! Each `bench_function` closure represents a caller that wants to reuse a path
//! across lookups: on the old API that requires `path.clone()`, which is exactly
//! the cost the borrowed-path change removes.
#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reactive_stores::{Store, StoreField, StorePath, StorePathSegment};

#[derive(Debug, Default, Store)]
struct Nested {
    level_a: LevelA,
}

#[derive(Debug, Default, Store)]
struct LevelA {
    level_b: LevelB,
}

#[derive(Debug, Default, Store)]
struct LevelB {
    level_c: LevelC,
}

#[derive(Debug, Default, Store)]
struct LevelC {
    level_d: LevelD,
}

#[derive(Debug, Default, Store)]
struct LevelD {
    value: i32,
}

fn deep_path() -> StorePath {
    let mut path = StorePath::new();
    for i in 0..12 {
        path.push(StorePathSegment::from(i));
    }
    path
}

fn bench(c: &mut Criterion) {
    let store = Store::new(Nested::default());
    let path = deep_path();

    // the hot path: `get_trigger` on an already-cached path
    c.bench_function("get_trigger", |b| {
        b.iter(|| black_box(store.get_trigger(&path)));
    });

    // the function called out in #4057, including the ancestor walk
    c.bench_function("triggers_for_path", |b| {
        b.iter(|| black_box(store.triggers_for_path(&path)));
    });

    c.bench_function("triggers_for_path_unkeyed", |b| {
        b.iter(|| black_box(store.triggers_for_path_unkeyed(&path)));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
