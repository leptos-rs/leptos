//! Minimal reproduction for leptos-rs/leptos#4610
//!
//! Rapidly toggling <Show> with on:click handlers triggers
//! "callback removed before attaching" panic in WebView2/Tauri.

use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    let show = RwSignal::new(true);
    let count = RwSignal::new(0);

    // Auto-trigger on load for automated testing (delay 300ms, then 10 cycles of 30 rapid toggles)
    let show_auto = show.clone();
    gloo_timers::callback::Timeout::new(300, move || {
        for cycle in 0..10 {
            for i in 0..30 {
                let s = show_auto.clone();
                gloo_timers::callback::Timeout::new(cycle * 150 + 3 + i * 3, move || {
                    s.update(|v| *v = !*v);
                })
                .forget();
            }
        }
    })
    .forget();

    // Timer-based rapid toggle - fires 30 toggles at 5ms intervals to trigger mount/unmount churn
    let start_rapid_toggle = move |_| {
        let show = show.clone();
        for i in 0..30 {
            let show = show.clone();
            gloo_timers::callback::Timeout::new(5 + i * 5, move || {
                show.update(|s| *s = !*s);
            })
            .forget();
        }
    };

    view! {
        <div style="font-family: sans-serif; padding: 20px;">
            <h1>"WebView2 Callback Panic Repro"</h1>
            <p>"Click 'Toggle rapidly' to mount/unmount Show boundary with on:click handlers."</p>

            <div style="margin: 20px 0;">
                <button
                    on:click=start_rapid_toggle
                    style="padding: 10px 20px; font-size: 16px; cursor: pointer;"
                >
                    "Toggle rapidly (30x @ 5ms)"
                </button>
                <button
                    on:click=move |_| show.set(!show.get())
                    style="padding: 10px 20px; font-size: 16px; cursor: pointer; margin-left: 10px;"
                >
                    "Toggle once"
                </button>
            </div>

            <Show
                when=move || show.get()
                fallback=|| view! { <p>"Hidden"</p> }
            >
                <div style="padding: 20px; background: #f0f0f0; border-radius: 8px; margin-top: 20px;">
                    <p>"Visible content with on:click handler:"</p>
                    <button
                        on:click=move |_| count.update(|c| *c += 1)
                        style="padding: 8px 16px; cursor: pointer;"
                    >
                        "Count: " {move || count.get()}
                    </button>
                </div>
            </Show>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
