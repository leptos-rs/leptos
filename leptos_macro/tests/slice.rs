use leptos::prelude::RwSignal;
use leptos_macro::slice;

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

    let (_, _) = slice!(outer_signal.count);

    let (_, _) = slice!(outer_signal.inner.inner_count);
    let (_, _) = slice!(outer_signal.inner.inner_tuple.0);
}

#[test]
fn red() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/slice/red.rs")
}
