use leptos_macro::slice;
use leptos::prelude::RwSignal;

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

    let (_, _) = slice!();

    let (_, _) = slice!(outer_signal);

    let (_, _) = slice!(outer_signal.);

    let (_, _) = slice!(outer_signal.inner.);
}
