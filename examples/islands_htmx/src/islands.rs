use crate::collect::collected;
use leptos::{ev, prelude::*};

/// A counter with state of its own. It arrives in a fragment htmx swaps in, so it must be woken.
#[island]
pub fn Counter() -> impl IntoView {
    collected(|| {
        let count = RwSignal::new(0);
        view! {
            <button on:click=move |_| *count.write() += 1>"Clicks: " {count}</button>
        }
    })
}

/// Shows the last key pressed. Its window listener is removed when the collector frees the island.
#[island]
pub fn KeyLog() -> impl IntoView {
    collected(|| {
        let key = RwSignal::new(String::new());
        let listener =
            window_event_listener(ev::keydown, move |ev| key.set(ev.key()));
        on_cleanup(move || listener.remove());
        view! { <p>"Last key: " {key}</p> }
    })
}
