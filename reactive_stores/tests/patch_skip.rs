//! Regression test: a `#[store(skip)]` field is opted out of patching. Its
//! type need not implement `PatchField`, and `patch` leaves it untouched.

use reactive_graph::{owner::Owner, traits::ReadUntracked};
use reactive_stores::{Patch, Store};

// Deliberately implements neither `PatchField` nor `PartialEq`.
#[derive(Debug, Default)]
struct Handle(i32);

#[derive(Debug, Store, reactive_stores::Patch, Default)]
struct State {
    #[store(skip)]
    handle: Handle,
    value: i32,
}

#[test]
fn skip_field_is_not_patched() {
    let owner = Owner::new();
    owner.set();

    let store = Store::new(State {
        handle: Handle(7),
        value: 1,
    });

    store.patch(State {
        handle: Handle(99),
        value: 42,
    });

    let guard = store.read_untracked();
    // The skipped field keeps its original value; the rest is patched.
    assert_eq!(guard.handle.0, 7);
    assert_eq!(guard.value, 42);
}
