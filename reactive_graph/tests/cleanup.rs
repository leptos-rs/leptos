use reactive_graph::{
    computed::Memo,
    owner::{on_cleanup, Owner},
    signal::{RwSignal, Trigger},
    traits::{Dispose, GetUntracked, Track},
};
use std::sync::Arc;

#[test]
fn cleanup_on_dispose() {
    let owner = Owner::new();
    owner.set();

    struct ExecuteOnDrop(Option<Box<dyn FnOnce() + Send + Sync>>);

    impl ExecuteOnDrop {
        fn new(f: impl FnOnce() + Send + Sync + 'static) -> Self {
            Self(Some(Box::new(f)))
        }
    }
    impl Drop for ExecuteOnDrop {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    let trigger = Trigger::new();

    println!("STARTING");

    let memo = Memo::new(move |_| {
        trigger.track();

        // An example of why you might want to do this is that
        // when something goes out of reactive scope you want it to be cleaned up.
        // The cleaning up might have side effects, and those side effects might cause
        // re-renders where new `on_cleanup` are registered.
        let on_drop = ExecuteOnDrop::new(|| {
            on_cleanup(|| println!("Nested cleanup in progress."))
        });

        on_cleanup(move || {
            println!("Cleanup in progress.");
            drop(on_drop)
        });
    });
    println!("Memo 1: {:?}", memo);
    memo.get_untracked(); // First cleanup registered.

    memo.dispose(); // Cleanup not run here.

    println!("Cleanup should have been executed.");

    let memo = Memo::new(move |_| {
        // New cleanup registered. It'll panic here.
        on_cleanup(move || println!("Test passed."));
    });
    println!("Memo 2: {:?}", memo);
    println!("^ Note how the memos have the same key (different versions).");
    memo.get_untracked(); // First cleanup registered.

    println!("Test passed.");

    memo.dispose();
}

#[test]
fn leak_on_dispose() {
    let owner = Owner::new();
    owner.set();

    let trigger = Trigger::new();

    let value = Arc::new(());
    let weak = Arc::downgrade(&value);

    let memo = Memo::new(move |_| {
        trigger.track();

        RwSignal::new(value.clone());
    });

    memo.get_untracked();

    memo.dispose();

    assert!(weak.upgrade().is_none()); // Should have been dropped.
}
