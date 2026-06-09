//! Regression test: fields of an enum variant must each have a distinct
//! reactive path, so that writing one field does not wake subscribers of a
//! sibling field.

use reactive_graph::{
    effect::Effect,
    owner::Owner,
    traits::{Read, Track, Write},
};
use reactive_stores::Store;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Store, Default)]
enum Form {
    #[default]
    Empty,
    Loaded {
        name: String,
        age: u32,
    },
}

#[derive(Store, Default)]
struct State {
    form: Form,
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_micros(1)).await;
}

#[tokio::test]
async fn writing_one_variant_field_does_not_wake_sibling() {
    _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let store = Store::new(State {
        form: Form::Loaded {
            name: "x".into(),
            age: 1,
        },
    });

    let name_count = Arc::new(AtomicUsize::new(0));
    let age_count = Arc::new(AtomicUsize::new(0));

    let name_sf = store.form().loaded_name().unwrap();
    let age_sf = store.form().loaded_age().unwrap();

    Effect::new_sync({
        let c = Arc::clone(&name_count);
        move |_: Option<()>| {
            let _ = name_sf.read();
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    Effect::new_sync({
        let c = Arc::clone(&age_count);
        move |_: Option<()>| {
            let _ = age_sf.read();
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;

    // Write only to `age`.
    *store.form().loaded_age().unwrap().write() = 99;
    tick().await;

    // The `name` effect must not have re-run (initial run only).
    assert_eq!(
        name_count.load(Ordering::Relaxed),
        1,
        "name effect woke on an age-only write (path collision)"
    );
    // The `age` effect re-ran: initial + after the write.
    assert_eq!(age_count.load(Ordering::Relaxed), 2);
}

// Two single-field variants. Each field must get a path segment that is unique
// across the *whole* enum, not just within its own variant. Without the
// per-variant base offset, `A::a` and `B::b` both collapse onto segment `0`,
// so writing one falsely wakes a subscriber of the other.
#[derive(Store, Default)]
enum Multi {
    #[default]
    Empty,
    A {
        a: u32,
    },
    B {
        b: u32,
    },
}

#[derive(Store, Default)]
struct MultiState {
    multi: Multi,
}

#[tokio::test]
async fn writing_one_variant_field_does_not_wake_a_different_variants_sibling()
{
    _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    let store = Store::new(MultiState {
        multi: Multi::A { a: 1 },
    });

    let a_count = Arc::new(AtomicUsize::new(0));

    // Subfield for `A::a`. We only *track* it (never read), so its subscription
    // can outlive a switch to a different variant -- reading a non-matching
    // variant would panic on the "accessed an enum field that is no longer
    // matched" guard.
    let a_sf = store.multi().a_a().unwrap();

    Effect::new_sync({
        let c = Arc::clone(&a_count);
        move |_: Option<()>| {
            a_sf.track();
            c.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;
    assert_eq!(a_count.load(Ordering::Relaxed), 1, "initial run");

    // Switch `A` -> `B`. This writes the whole `multi` field and legitimately
    // wakes the tracker once, via the ancestor `this@[multi]` trigger.
    *store.multi().write() = Multi::B { b: 0 };
    tick().await;
    let after_switch = a_count.load(Ordering::Relaxed);
    assert_eq!(
        after_switch, 2,
        "the variant switch should wake the tracker exactly once"
    );

    // Now write the *other* variant's field. `B::b` has a path distinct from
    // `A::a`, so these writes must NOT wake the `A::a` tracker. If the two
    // fields shared segment `0`, each write here would falsely wake it.
    *store.multi().b_b().unwrap().write() = 7;
    tick().await;
    *store.multi().b_b().unwrap().write() = 8;
    tick().await;

    assert_eq!(
        a_count.load(Ordering::Relaxed),
        after_switch,
        "writing a different variant's field woke a cross-variant subscriber \
         (path collision across variants)"
    );
}
