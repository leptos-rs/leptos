//! Regression test for `LocalStorage` access from the wrong thread.
//!
//! `StoredValue<T, LocalStorage>` stores its value in a `SendWrapper`, which
//! may only be dereferenced on the thread that created it. The `try_*` methods
//! are documented to return `None` when the value "can[not] be accessed from
//! this thread" — they must not panic.

use reactive_graph::{
    owner::{LocalStorage, Owner, StoredValue},
    traits::{ReadValue, UpdateValue, WriteValue},
};

#[test]
fn local_storage_try_access_off_thread_returns_none() {
    let owner = Owner::new();
    owner.set();

    let value: StoredValue<String, LocalStorage> =
        StoredValue::new_local("created on thread A".to_string());

    // On the creating thread, access works.
    assert_eq!(
        value.try_read_value().as_deref(),
        Some(&"created on thread A".to_string())
    );

    // On another thread every `try_*` access must report `None`, not panic.
    // A clone of the owner is re-activated on the spawned thread so it has a
    // live arena (required under the `sandboxed-arenas` feature); the value
    // itself still lives in a `SendWrapper` bound to thread A, so access must
    // fail. The original owner stays alive here to keep the arena valid for the
    // final on-thread assertion.
    let owner_b = owner.clone();
    let results = std::thread::spawn(move || {
        owner_b.set();
        let read = value.try_read_value().is_none();
        let updated = value.try_update_value(|s| s.push('!')).is_none();
        let written = value.try_write_value().is_none();
        (read, updated, written)
    })
    .join()
    .expect("accessing LocalStorage off-thread must not panic");

    assert_eq!(
        results,
        (true, true, true),
        "off-thread try_* access should return None"
    );

    // The value on the original thread is untouched by the failed off-thread
    // attempts.
    assert_eq!(
        value.try_read_value().as_deref(),
        Some(&"created on thread A".to_string())
    );
}
