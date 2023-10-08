use leptos_macro::Lens;

#[derive(Lens)]
struct GlobalState {
    count: i32,
}

fn main() {
    let state = create_rw_signal(GlobalState::default());
    let _ = GlobalState::count(state);
}
