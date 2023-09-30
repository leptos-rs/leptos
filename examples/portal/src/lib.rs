use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    let (show_overlay, set_show_overlay) = create_signal(false);

    view! {
        <div>
            <button on:click=move |_| set_show_overlay(true)>
                Show Overlay
            </button>

            <Show when=show_overlay fallback=|| ()>
                <div>Show</div>
                <Portal>
                    <div style="position: fixed; z-index: 10; width: 100vw; height: 100vh; top: 0; left: 0; background: rgba(0, 0, 0, 0.5)">
                        This is in the body element
                        <button on:click=move |_| set_show_overlay(false)>
                            Close Overlay
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}
