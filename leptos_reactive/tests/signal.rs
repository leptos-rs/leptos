use leptos_reactive::{with_root_scope, Scope};

#[test]
fn basic_signal() {
    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        assert_eq!(a(), 0);
        set_a(|a| *a = 5);
        assert_eq!(a(), 5);
    });
    unsafe { d.dispose() }
}

#[test]
fn derived_signals() {
    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = move || a() + b();
        assert_eq!(c(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
    });
    unsafe { d.dispose() }
}

#[test]
fn basic_memo() {
    let d = with_root_scope(|cx| {
        let a = cx.create_memo(|| 5);
        assert_eq!(a(), 5);
    });
    unsafe { d.dispose() }
}

#[test]
fn memo_with_computed_value() {
    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = cx.create_memo(move || a() + b());
        assert_eq!(c(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
    });
    unsafe { d.dispose() }
}

#[test]
fn nested_memos() {
    let d = with_root_scope(|cx| {
        let (a, set_a) = cx.create_signal(0);
        let (b, set_b) = cx.create_signal(0);
        let c = cx.create_memo(move || a() + b());
        let d = cx.create_memo(move || c() * 2);
        let e = cx.create_memo(move || d() + 1);
        assert_eq!(d(), 0);
        set_a(|a| *a = 5);
        assert_eq!(c(), 5);
        assert_eq!(d(), 10);
        assert_eq!(e(), 11);
        set_b(|b| *b = 1);
        assert_eq!(c(), 6);
        assert_eq!(d(), 12);
        assert_eq!(e(), 13);
    });
    unsafe { d.dispose() }
}
