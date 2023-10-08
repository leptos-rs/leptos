use leptos::*;
use leptos_macro::Lens;

#[derive(Lens)]
struct GlobalState {
    count: i32,
}

fn main() {
    let _ = create_runtime();

    let state = create_rw_signal(GlobalState { count: 0 });
    let _ = GlobalState::count(state);
}
