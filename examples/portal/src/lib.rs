use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    let (show_overlay, set_show_overlay) = create_signal(false);
    let (show_inside_overlay, set_show_inside_overlay) = create_signal(false);

    view! {
        <div>
            <button on:click=move |_| set_show_overlay(true)>
                Show Overlay
            </button>

            <Show when=show_overlay fallback=|| ()>
                <div>Show</div>
                <Portal>
                    <div style="position: fixed; z-index: 10; width: 100vw; height: 100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.8); color: white;">
                        <p>This is in the body element</p>
                        <button on:click=move |_| set_show_overlay(false)>
                            Close Overlay
                        </button>
                        <button on:click=move |_| set_show_inside_overlay(!show_inside_overlay())>
                            Toggle inner
                        </button>

                        <Show when=show_inside_overlay fallback=|| view! { "Hidden" }>
                            Visible
                        </Show>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}
