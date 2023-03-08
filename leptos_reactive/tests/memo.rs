#[cfg(not(feature = "stable"))]
use leptos_reactive::{
    create_memo, create_runtime, create_scope, create_signal,
};

#[cfg(not(feature = "stable"))]
#[test]
fn basic_memo() {
    create_scope(create_runtime(), |cx| {
        let a = create_memo(cx, |_| 5);
        assert_eq!(a(), 5);
    })
    .dispose()
}

#[cfg(not(feature = "stable"))]
#[test]
fn memo_with_computed_value() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, 0);
        let (b, set_b) = create_signal(cx, 0);
        let c = create_memo(cx, move |_| a() + b());
        assert_eq!(c(), 0);
        set_a(5);
        assert_eq!(c(), 5);
        set_b(1);
        assert_eq!(c(), 6);
    })
    .dispose()
}

#[cfg(not(feature = "stable"))]
#[test]
fn nested_memos() {
    create_scope(create_runtime(), |cx| {
        let (a, set_a) = create_signal(cx, 0); // 1
        let (b, set_b) = create_signal(cx, 0); // 2
        let c = create_memo(cx, move |_| a() + b()); // 3
        let d = create_memo(cx, move |_| c() * 2); // 4
        let e = create_memo(cx, move |_| d() + 1); // 5
        assert_eq!(d(), 0);
        set_a(5);
        assert_eq!(e(), 11);
        assert_eq!(d(), 10);
        assert_eq!(c(), 5);
        set_b(1);
        assert_eq!(e(), 13);
        assert_eq!(d(), 12);
        assert_eq!(c(), 6);
    })
    .dispose()
}

#[cfg(not(feature = "stable"))]
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
                a() + b() + c()
            }
        });

        // initially the memo has not been called at all, because it's lazy
        assert_eq!(call_count.get(), 0);

        // here we access the value a bunch of times
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);

        // we've still only called the memo calculation once
        assert_eq!(call_count.get(), 1);

        // and we only call it again when an input changes
        set_a(1);
        assert_eq!(c(), 1);
        assert_eq!(call_count.get(), 2);
    })
    .dispose()
}

#[cfg(not(feature = "stable"))]
#[test]
fn diamond_problem() {
    use std::{cell::Cell, rc::Rc};

    create_scope(create_runtime(), |cx| {
        let (name, set_name) = create_signal(cx, "Greg Johnston".to_string());
        let first = create_memo(cx, move |_| {
            name().split_whitespace().next().unwrap().to_string()
        });
        let last = create_memo(cx, move |_| {
            name().split_whitespace().nth(1).unwrap().to_string()
        });

        let combined_count = Rc::new(Cell::new(0));
        let combined = create_memo(cx, {
            let combined_count = Rc::clone(&combined_count);
            move |_| {
                combined_count.set(combined_count.get() + 1);
                format!("{} {}", first(), last())
            }
        });

        assert_eq!(first(), "Greg");
        assert_eq!(last(), "Johnston");

        set_name("Will Smith".to_string());
        assert_eq!(first(), "Will");
        assert_eq!(last(), "Smith");
        assert_eq!(combined(), "Will Smith");
        // should not have run the memo logic twice, even
        // though both paths have been updated
        assert_eq!(combined_count.get(), 1);
    })
    .dispose()
}
