use leptos_reactive::{
    batch, create_isomorphic_effect, create_memo, create_runtime,
    create_rw_signal, create_signal, untrack, SignalGet, SignalSet,
};

#[test]
fn effect_runs() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = create_runtime();

    let (a, set_a) = create_signal(-1);

    // simulate an arbitrary side effect
    let b = Rc::new(RefCell::new(String::new()));

    create_isomorphic_effect({
        let b = b.clone();
        move |_| {
            let formatted = format!("Value is {}", a.get());
            *b.borrow_mut() = formatted;
        }
    });

    assert_eq!(b.borrow().as_str(), "Value is -1");

    set_a.set(1);

    assert_eq!(b.borrow().as_str(), "Value is 1");

    runtime.dispose();
}

#[test]
fn effect_tracks_memo() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = create_runtime();
    let (a, set_a) = create_signal(-1);
    let b = create_memo(move |_| format!("Value is {}", a.get()));

    // simulate an arbitrary side effect
    let c = Rc::new(RefCell::new(String::new()));

    create_isomorphic_effect({
        let c = c.clone();
        move |_| {
            *c.borrow_mut() = b.get();
        }
    });

    assert_eq!(b.get().as_str(), "Value is -1");
    assert_eq!(c.borrow().as_str(), "Value is -1");

    set_a.set(1);

    assert_eq!(b.get().as_str(), "Value is 1");
    assert_eq!(c.borrow().as_str(), "Value is 1");

    runtime.dispose();
}

#[test]
fn untrack_mutes_effect() {
    use std::{cell::RefCell, rc::Rc};

    let runtime = create_runtime();

    let (a, set_a) = create_signal(-1);

    // simulate an arbitrary side effect
    let b = Rc::new(RefCell::new(String::new()));

    create_isomorphic_effect({
        let b = b.clone();
        move |_| {
            let formatted = format!("Value is {}", untrack(move || a.get()));
            *b.borrow_mut() = formatted;
        }
    });

    assert_eq!(a.get(), -1);
    assert_eq!(b.borrow().as_str(), "Value is -1");

    set_a.set(1);

    assert_eq!(a.get(), 1);
    assert_eq!(b.borrow().as_str(), "Value is -1");

    runtime.dispose();
}

#[test]
fn batching_actually_batches() {
    use std::{cell::Cell, rc::Rc};

    let runtime = create_runtime();

    let first_name = create_rw_signal("Greg".to_string());
    let last_name = create_rw_signal("Johnston".to_string());

    // simulate an arbitrary side effect
    let count = Rc::new(Cell::new(0));

    create_isomorphic_effect({
        let count = count.clone();
        move |_| {
            _ = first_name.get();
            _ = last_name.get();

            count.set(count.get() + 1);
        }
    });

    // runs once initially
    assert_eq!(count.get(), 1);

    // individual updates run effect once each
    first_name.set("Alice".to_string());
    assert_eq!(count.get(), 2);

    last_name.set("Smith".to_string());
    assert_eq!(count.get(), 3);

    // batched effect only runs twice
    batch(move || {
        first_name.set("Bob".to_string());
        last_name.set("Williams".to_string());
    });
    assert_eq!(count.get(), 4);

    runtime.dispose();
}
