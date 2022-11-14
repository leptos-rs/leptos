use cfg_if::cfg_if;
use leptos::*;
mod counters;
use crate::counters::*;

#[component]
pub fn App(cx: Scope) -> Element {
    let (value, set_value) = create_signal(cx, 0);

    view! { cx,
        <div>
            <button on:click=move |_| set_value(0)>"Clear"</button>
            <button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
        </div>
    }
}

// Needs to be in lib.rs AFAIK because wasm-bindgen needs us to be compiling a lib. I may be wrong.
cfg_if! {
    if #[cfg(feature = "hydrate")] {
        #[wasm_bindgen]
        pub fn main() {
            console_error_panic_hook::set_once();
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();

            leptos::hydrate(body().unwrap(), |cx| {
                view! { cx,  <Counters/> }
            });
        }
    }
}
