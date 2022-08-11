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
