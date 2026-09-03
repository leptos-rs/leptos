//! Regression tests for the `Hash`/`Eq` contract on pointer-identity handles.
//!
//! These types implement `PartialEq` via `Arc::ptr_eq`, so two clones that
//! share the same allocation compare equal. The `Hash` impl must therefore
//! hash the shared heap pointer, not the address of a stack temporary.

use reactive_graph::{
    computed::ArcMemo,
    owner::{ArcStoredValue, Owner},
    signal::{ArcReadSignal, ArcRwSignal, ArcWriteSignal},
    traits::Get,
};
use std::{
    collections::HashMap,
    hash::{BuildHasher, BuildHasherDefault, Hash},
};

fn hash_of<T: Hash>(t: &T) -> u64 {
    BuildHasherDefault::<rustc_hash::FxHasher>::default().hash_one(t)
}

/// `a == a.clone()` must imply `hash(a) == hash(a.clone())`, and a `HashMap`
/// keyed by the handle must find an entry inserted under a clone.
fn assert_hash_eq_contract<T>(a: T)
where
    T: Clone + Eq + Hash + std::fmt::Debug,
{
    let b = a.clone();
    assert_eq!(a, b, "clones must compare equal");
    assert_eq!(
        hash_of(&a),
        hash_of(&b),
        "Hash/Eq contract violated: equal values hashed differently"
    );

    let mut map: HashMap<T, &'static str> = HashMap::new();
    map.insert(a, "value");
    assert_eq!(map.get(&b), Some(&"value"), "lookup with a clone must hit");
}

#[test]
fn arc_rw_signal_hash_eq_contract() {
    assert_hash_eq_contract(ArcRwSignal::new(0_u32));
}

#[test]
fn arc_read_signal_hash_eq_contract() {
    let (read, _write) = reactive_graph::signal::arc_signal(0_u32);
    let _ = read.get();
    assert_hash_eq_contract::<ArcReadSignal<u32>>(read);
}

#[test]
fn arc_write_signal_hash_eq_contract() {
    let (_read, write) = reactive_graph::signal::arc_signal(0_u32);
    assert_hash_eq_contract::<ArcWriteSignal<u32>>(write);
}

#[test]
fn arc_memo_hash_eq_contract() {
    let owner = Owner::new();
    owner.set();
    let memo = ArcMemo::new(|_| 0_u32);
    assert_hash_eq_contract(memo);
}

#[test]
fn arc_stored_value_hash_eq_contract() {
    let owner = Owner::new();
    owner.set();
    assert_hash_eq_contract(ArcStoredValue::new(0_u32));
}
