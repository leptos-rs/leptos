use leptos::{control_flow::Show, portal::Portal, prelude::*};

#[component]
pub fn App() -> impl IntoView {
    let (show_overlay, set_show_overlay) = signal(false);
    let (show_inside_overlay, set_show_inside_overlay) = signal(false);

    view! {
        <div>
            <button id="btn-show" on:click=move |_| set_show_overlay.set(true)>
                "Show Overlay"
            </button>

            <Show when=move || show_overlay.get() fallback=|| ()>
                <div>Show</div>
                <Portal mount=document().get_element_by_id("app").unwrap()>
                    <div style="position: fixed; z-index: 10; width: 100vw; height: 100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: white;">
                        <p>This is in the body element</p>
                        <button id="btn-hide" on:click=move |_| set_show_overlay.set(false)>
                            "Close Overlay"
                        </button>
                        <button
                            id="btn-toggle"
                            on:click=move |_| {
                                set_show_inside_overlay.set(!show_inside_overlay.get())
                            }
                        >
                            "Toggle inner"
                        </button>

                        <Show when=move || show_inside_overlay.get() fallback=|| view! { "Hidden" }>
                            "Visible"
                        </Show>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}
