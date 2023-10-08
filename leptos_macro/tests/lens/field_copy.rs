use leptos::*;
use leptos_macro::Lens;

fn single_field() {
    #[derive(Default, Lens)]
    struct GlobalState {
        count: i32,
    }

    let _ = create_runtime();

    let state = create_rw_signal(GlobalState::default());
    let _ = GlobalState::count(state);
}

fn multiple_fields() {
    #[derive(Default, Lens)]
    struct GlobalState {
        count: i32,
        age: u32,
    }

    let _ = create_runtime();

    let state = create_rw_signal(GlobalState::default());
    let _ = GlobalState::count(state);
}

fn main() {
    single_field();
    multiple_fields();
}
