use leptos_reactive::{
    create_isomorphic_effect, create_memo, create_runtime, create_rw_signal,
    create_scope, create_signal, SignalGet, SignalSet,
};

#[test]
fn effect_runs() {
    use std::{cell::RefCell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_isomorphic_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted = format!("Value is {}", a.get());
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a.set(1);

        assert_eq!(b.borrow().as_str(), "Value is 1");
    })
    .dispose()
}

#[test]
fn effect_tracks_memo() {
    use std::{cell::RefCell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);
        let b = create_memo(cx, move |_| format!("Value is {}", a.get()));

        // simulate an arbitrary side effect
        let c = Rc::new(RefCell::new(String::new()));

        create_isomorphic_effect(cx, {
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
    })
    .dispose()
}

#[test]
fn untrack_mutes_effect() {
    use std::{cell::RefCell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, -1);

        // simulate an arbitrary side effect
        let b = Rc::new(RefCell::new(String::new()));

        create_isomorphic_effect(cx, {
            let b = b.clone();
            move |_| {
                let formatted =
                    format!("Value is {}", cx.untrack(move || a.get()));
                *b.borrow_mut() = formatted;
            }
        });

        assert_eq!(a.get(), -1);
        assert_eq!(b.borrow().as_str(), "Value is -1");

        set_a.set(1);

        assert_eq!(a.get(), 1);
        assert_eq!(b.borrow().as_str(), "Value is -1");
    })
    .dispose()
}

#[test]
fn batching_actually_batches() {
    use std::{cell::Cell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let first_name = create_rw_signal(cx, "Greg".to_string());
        let last_name = create_rw_signal(cx, "Johnston".to_string());

        // simulate an arbitrary side effect
        let count = Rc::new(Cell::new(0));

        create_isomorphic_effect(cx, {
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
        cx.batch(move || {
            first_name.set("Bob".to_string());
            last_name.set("Williams".to_string());
        });
        assert_eq!(count.get(), 4);
    })
    .dispose()
}
