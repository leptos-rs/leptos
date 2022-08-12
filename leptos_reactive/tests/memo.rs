use leptos_reactive::create_scope;

#[test]
fn basic_memo() {
    create_scope(|cx| {
        let a = cx.create_memo(|_| 5);
        assert_eq!(a(), 5);
    })
    .dispose()
}

#[test]
fn memo_with_computed_value() {
    create_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = cx.create_memo(move |_| a() + b());
        assert_eq!(c(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
    })
    .dispose()
}

#[test]
fn nested_memos() {
    create_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = cx.create_memo(move |_| a() + b());
        let d = cx.create_memo(move |_| c() * 2);
        let e = cx.create_memo(move |_| d() + 1);
        assert_eq!(d(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        assert_eq!(d(), 10);
        assert_eq!(e(), 11);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
        assert_eq!(d(), 12);
        assert_eq!(e(), 13);
    })
    .dispose()
}

#[test]
fn memo_runs_only_when_inputs_change() {
    use std::{cell::Cell, rc::Rc};

    create_scope(|cx| {
        let call_count = Rc::new(Cell::new(0));
        let (a, set_a) = cx.create_signal(0);
        let (b, _) = cx.create_signal(0);
        let (c, _) = cx.create_signal(0);

        // pretend that this is some kind of expensive computation and we need to access its its value often
        // we could do this with a derived signal, but that would re-run the computation
        // memos should only run when their inputs actually change: this is the only point
        let c = cx.create_memo({
            let call_count = call_count.clone();
            move |_| {
                call_count.set(call_count.get() + 1);
                a() + b() + c()
            }
        });

        assert_eq!(call_count.get(), 1);

        // here we access the value a bunch of times
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);
        assert_eq!(c(), 0);

        // we've still only called the memo calculation once
        assert_eq!(call_count.get(), 1);

        // and we only call it again when an input changes
        set_a(|n| *n = 1);
        assert_eq!(c(), 1);
        assert_eq!(call_count.get(), 2);
    })
    .dispose()
}
