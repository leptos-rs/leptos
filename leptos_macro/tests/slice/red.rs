use leptos::{create_runtime, create_rw_signal};
use leptos_macro::slice;

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
    let _ = create_runtime();

    let outer_signal = create_rw_signal(OuterState::default());

    let (_, _) = slice!();

    let (_, _) = slice!(outer_signal);

    let (_, _) = slice!(outer_signal.);

    let (_, _) = slice!(outer_signal.inner.);
}
