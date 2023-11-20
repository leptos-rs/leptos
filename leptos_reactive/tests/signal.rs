use leptos_reactive::*;

#[test]
fn basic_signal() {
    let runtime = create_runtime();

    let (a, set_a) = create_signal(0);
    assert_eq!(a.get(), 0);
    set_a.set(5);
    assert_eq!(a.get(), 5);

    runtime.dispose();
}

#[test]
fn derived_signals() {
    let runtime = create_runtime();

    let (a, set_a) = create_signal(0);
    let (b, set_b) = create_signal(0);
    let c = move || a.get() + b.get();
    assert_eq!(c(), 0);
    set_a.set(5);
    assert_eq!(c(), 5);
    set_b.set(1);
    assert_eq!(c(), 6);

    runtime.dispose();
}
