use leptos::{prelude::*, subsecond::connect_to_hot_patch_messages};

fn main() {
    // connect to DX CLI and patch the WASM binary whenever we receive a message
    connect_to_hot_patch_messages();

    // wrapping App here in a closure so we can hot-reload it, because we only do that
    // for reactive views right now. changing anything will re-run App and update the view
    mount_to_body(|| App);
}

fn App() -> impl IntoView {
    let msg = RwSignal::new("hello, world!".to_string());
    view! {
        <p>{msg}</p>
        <button on:click=move |_| {
            msg.set("wow!".to_string());
        }>
            "a cool button"
        </button>
    }
    .into_any()
}
