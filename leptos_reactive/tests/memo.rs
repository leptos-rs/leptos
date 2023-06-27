use leptos_reactive::*;

#[test]
fn basic_memo() {
    create_scope(create_runtime(), |cx| {
        let a = create_memo(cx, |_| 5);
        assert_eq!(a.get(), 5);
    })
    .dispose()
}

#[test]
fn signal_with_untracked() {
    use leptos_reactive::SignalWithUntracked;

    create_scope(create_runtime(), |cx| {
        let m = create_memo(cx, move |_| 5);
        let copied_out = m.with_untracked(|value| *value);
        assert_eq!(copied_out, 5);
    })
    .dispose()
}

#[test]
fn signal_get_untracked() {
    use leptos_reactive::SignalGetUntracked;

    create_scope(create_runtime(), |cx| {
        let m = create_memo(cx, move |_| "memo".to_owned());
        let cloned_out = m.get_untracked();
        assert_eq!(cloned_out, "memo".to_owned());
    })
    .dispose()
}

#[test]
fn memo_with_computed_value() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, 0);
        let (b, set_b) = create_signal(cx, 0);
        let c = create_memo(cx, move |_| a.get() + b.get());
        assert_eq!(c.get(), 0);
        set_a.set(5);
        assert_eq!(c.get(), 5);
        set_b.set(1);
        assert_eq!(c.get(), 6);
    })
    .dispose()
}

#[test]
fn nested_memos() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, 0); // 1
        let (b, set_b) = create_signal(cx, 0); // 2
        let c = create_memo(cx, move |_| a.get() + b.get()); // 3
        let d = create_memo(cx, move |_| c.get() * 2); // 4
        let e = create_memo(cx, move |_| d.get() + 1); // 5
        assert_eq!(d.get(), 0);
        set_a.set(5);
        assert_eq!(e.get(), 11);
        assert_eq!(d.get(), 10);
        assert_eq!(c.get(), 5);
        set_b.set(1);
        assert_eq!(e.get(), 13);
        assert_eq!(d.get(), 12);
        assert_eq!(c.get(), 6);
    })
    .dispose()
}

#[test]
fn memo_runs_only_when_inputs_change() {
    use std::{cell::Cell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let call_count = Rc::new(Cell::new(0));
        let (a, set_a) = create_signal(cx, 0);
        let (b, _) = create_signal(cx, 0);
        let (c, _) = create_signal(cx, 0);

        // pretend that this is some kind of expensive computation and we need to access its its value often
        // we could do this with a derived signal, but that would re-run the computation
        // memos should only run when their inputs actually change: this is the only point
        let c = create_memo(cx, {
            let call_count = call_count.clone();
            move |_| {
                call_count.set(call_count.get() + 1);
                a.get() + b.get() + c.get()
            }
        });

        // initially the memo has not been called at all, because it's lazy
        assert_eq!(call_count.get(), 0);

        // here we access the value a bunch of times
        assert_eq!(c.get(), 0);
        assert_eq!(c.get(), 0);
        assert_eq!(c.get(), 0);
        assert_eq!(c.get(), 0);
        assert_eq!(c.get(), 0);

        // we've still only called the memo calculation once
        assert_eq!(call_count.get(), 1);

        // and we only call it again when an input changes
        set_a.set(1);
        assert_eq!(c.get(), 1);
        assert_eq!(call_count.get(), 2);
    })
    .dispose()
}

#[test]
fn diamond_problem() {
    use std::{cell::Cell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (name, set_name) = create_signal(cx, "Greg Johnston".to_string());
        let first = create_memo(cx, move |_| {
            name.get().split_whitespace().next().unwrap().to_string()
        });
        let last = create_memo(cx, move |_| {
            name.get().split_whitespace().nth(1).unwrap().to_string()
        });

        let combined_count = Rc::new(Cell::new(0));
        let combined = create_memo(cx, {
            let combined_count = Rc::clone(&combined_count);
            move |_| {
                combined_count.set(combined_count.get() + 1);
                format!("{} {}", first.get(), last.get())
            }
        });

        assert_eq!(first.get(), "Greg");
        assert_eq!(last.get(), "Johnston");

        set_name.set("Will Smith".to_string());
        assert_eq!(first.get(), "Will");
        assert_eq!(last.get(), "Smith");
        assert_eq!(combined.get(), "Will Smith");
        // should not have run the memo logic twice, even
        // though both paths have been updated
        assert_eq!(combined_count.get(), 1);
    })
    .dispose()
}

#[test]
fn dynamic_dependencies() {
    use leptos_reactive::create_isomorphic_effect;
    use std::{cell::Cell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (first, set_first) = create_signal(cx, "Greg");
        let (last, set_last) = create_signal(cx, "Johnston");
        let (use_last, set_use_last) = create_signal(cx, true);
        let name = create_memo(cx, move |_| {
            if use_last.get() {
                format!("{} {}", first.get(), last.get())
            } else {
                first.get().to_string()
            }
        });

        let combined_count = Rc::new(Cell::new(0));

        create_isomorphic_effect(cx, {
            let combined_count = Rc::clone(&combined_count);
            move |_| {
                _ = name.get();
                combined_count.set(combined_count.get() + 1);
            }
        });

        assert_eq!(combined_count.get(), 1);

        set_first.set("Bob");
        assert_eq!(name.get(), "Bob Johnston");

        assert_eq!(combined_count.get(), 2);

        set_last.set("Thompson");

        assert_eq!(combined_count.get(), 3);

        set_use_last.set(false);

        assert_eq!(name.get(), "Bob");
        assert_eq!(combined_count.get(), 4);

        assert_eq!(combined_count.get(), 4);
        set_last.set("Jones");
        assert_eq!(combined_count.get(), 4);
        set_last.set("Smith");
        assert_eq!(combined_count.get(), 4);
        set_last.set("Stevens");
        assert_eq!(combined_count.get(), 4);

        set_use_last.set(true);
        assert_eq!(name.get(), "Bob Stevens");
        assert_eq!(combined_count.get(), 5);
    })
    .dispose()
}
