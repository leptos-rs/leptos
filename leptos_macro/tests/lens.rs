#[test]
fn lens() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/lens/field_copy.rs");
}

use leptos::{create_rw_signal, expect_context, provide_context, RwSignal};
use leptos_macro::Lens;

#[derive(Default, Lens)]
struct GlobalState {
    count: i32,
}

fn main() {
    provide_context(create_rw_signal(GlobalState::default()));
    let state = expect_context::<RwSignal<GlobalState>>();

    let (read, write) = GlobalState::count(state);
}
