use leptos::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let one_second = create_resource(
        cx,
        || (),
        |_| async {
            async_timer::timed(async { 1 }, std::time::Duration::from_secs(1))
                .await
                .unwrap()
        },
    );

    view! { cx,
        <main>
            <p>"Hello, world!"</p>

        </main>
    }
}
