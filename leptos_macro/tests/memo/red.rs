use leptos::prelude::RwSignal;
use leptos_macro::memo;

#[derive(Default, PartialEq)]
pub struct OuterState {
    count: i32,
    inner: InnerState,
}

#[derive(Clone, PartialEq, Default)]
pub struct InnerState {
    inner_count: i32,
    inner_name: String,
}

fn main() {
    let outer_signal = RwSignal::new(OuterState::default());

    let _ = memo!();

    let _ = memo!(outer_signal);

    let _ = memo!(outer_signal.);

    let _ = memo!(outer_signal.inner.);
}
