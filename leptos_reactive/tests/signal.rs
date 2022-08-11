use leptos_reactive::create_scope;

#[test]
fn basic_signal() {
    create_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        assert_eq!(a(), 0);
        set_a(|a| *a = 5);
        assert_eq!(a(), 5);
    })
    .dispose()
}

#[test]
fn derived_signals() {
    create_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = move || a() + b();
        assert_eq!(c(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
    })
    .dispose()
}

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
