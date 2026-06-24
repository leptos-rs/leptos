//! Regression test: fields of an enum variant must each have a distinct
//! reactive path, so that writing one field does not wake subscribers of a
//! sibling field.

use reactive_graph::{
    effect::Effect,
    owner::Owner,
    traits::{Read, Write},
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
