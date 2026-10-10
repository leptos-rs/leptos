//! Regression test: the `Patch` derive supports enums.
//! - patching within the same variant updates fields in place and only
//!   notifies the fields that changed,
//! - patching across variants replaces the value and notifies subscribers.

use reactive_graph::{
    effect::Effect,
    owner::Owner,
    traits::{Get, Read, ReadUntracked},
};
use reactive_stores::{Patch, Store};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, PartialEq, Patch, Store, Default)]
enum Mode {
    #[default]
    Idle,
    Running {
        progress: u32,
        label: String,
    },
    Done(u32),
}

#[derive(Debug, Clone, PartialEq, Patch, Store, Default)]
struct State {
    mode: Mode,
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_micros(1)).await;
}

#[tokio::test]
async fn same_variant_patch_only_notifies_changed_field() {
    _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let store = Store::new(State {
        mode: Mode::Running {
            progress: 1,
            label: "a".into(),
        },
    });

    let progress_count = Arc::new(AtomicUsize::new(0));
    let label_count = Arc::new(AtomicUsize::new(0));

    let progress_sf = store.mode().running_progress().unwrap();
    let label_sf = store.mode().running_label().unwrap();

    Effect::new_sync({
        let c = Arc::clone(&progress_count);
        move |_: Option<()>| {
            let _ = progress_sf.read();
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    Effect::new_sync({
        let c = Arc::clone(&label_count);
        move |_: Option<()>| {
            let _ = label_sf.read();
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;

    // Patch within the same variant, changing only `progress`.
    store.patch(State {
        mode: Mode::Running {
            progress: 2,
            label: "a".into(),
        },
    });
    tick().await;

    assert_eq!(
        store.mode().running_progress().unwrap().get(),
        2,
        "progress should be patched in place"
    );
    assert_eq!(
        progress_count.load(Ordering::Relaxed),
        2,
        "progress effect should re-run after its field changed"
    );
    assert_eq!(
        label_count.load(Ordering::Relaxed),
        1,
        "label effect should not re-run when only progress changed"
    );
}

#[tokio::test]
async fn cross_variant_patch_replaces_and_notifies() {
    _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let store = Store::new(State { mode: Mode::Idle });

    let is_running_count = Arc::new(AtomicUsize::new(0));
    let is_running = Arc::new(AtomicUsize::new(0));

    Effect::new_sync({
        let c = Arc::clone(&is_running_count);
        let r = Arc::clone(&is_running);
        move |_: Option<()>| {
            r.store(store.mode().running() as usize, Ordering::Relaxed);
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;
    assert_eq!(is_running.load(Ordering::Relaxed), 0);

    // Patch across variants: Idle -> Running.
    store.patch(State {
        mode: Mode::Running {
            progress: 5,
            label: "x".into(),
        },
    });
    tick().await;

    assert_eq!(
        is_running_count.load(Ordering::Relaxed),
        2,
        "variant-matcher effect should re-run after a variant change"
    );
    assert_eq!(is_running.load(Ordering::Relaxed), 1);
    assert!(matches!(
        store.read_untracked().mode,
        Mode::Running { progress: 5, .. }
    ));
}

#[tokio::test]
async fn unnamed_variant_patches_in_place() {
    _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let store = Store::new(State {
        mode: Mode::Done(1),
    });
    store.patch(State {
        mode: Mode::Done(7),
    });
    assert!(matches!(store.read_untracked().mode, Mode::Done(7)));
}
