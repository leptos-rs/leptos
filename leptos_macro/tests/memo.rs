use leptos::prelude::RwSignal;
use leptos_macro::memo;

#[derive(Default)]
pub struct OuterState {
    count: i32,
    inner: InnerState,
}

#[derive(Clone, PartialEq, Default)]
pub struct InnerState {
    inner_count: i32,
    inner_tuple: InnerTuple,
}

#[derive(Clone, PartialEq, Default)]
pub struct InnerTuple(String);

#[test]
fn green() {
    let outer_signal = RwSignal::new(OuterState::default());

    let _ = memo!(outer_signal.count);

    let _ = memo!(outer_signal.inner.inner_count);
    let _ = memo!(outer_signal.inner.inner_tuple.0);
}

#[test]
fn red() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/memo/red.rs")
}
