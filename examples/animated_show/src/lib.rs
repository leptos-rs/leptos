use core::time::Duration;
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    let show = create_rw_signal(false);

    // the CSS classes in this example are just written directly inside the `index.html`
    view! {
        <div
            class="hover-me"
            on:mouseenter=move |_| show.set(true)
            on:mouseleave=move |_| show.set(false)
        >
            "Hover Me"
        </div>

        <AnimatedShow
            when=show
            // optional CSS class which will be applied if `when == true`
            show_class="fade-in-1000"
            // optional CSS class which will be applied if `when == false` and before the
            // `hide_delay` starts -> makes CSS unmount animations really easy
            hide_class="fade-out-1000"
            // the given unmount delay which should match your unmount animation duration
            hide_delay=Duration::from_millis(1000)
        >
            // provide any `Children` inside here
            <div class="here-i-am">
                "Here I Am!"
            </div>
        </AnimatedShow>
    }
}
