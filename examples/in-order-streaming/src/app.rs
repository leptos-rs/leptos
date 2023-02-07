use leptos::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let one_second = create_resource(
        cx,
        || (),
        |_| futures_timer::Delay::new(std::time::Duration::from_secs(1)),
    );

    view! { cx,
        <main>
            <p>"Hello, world!"</p>
            <Suspense fallback=move || view! { cx, <p>"Loading..."</p> }>
                {move || one_second.read().map(|_| view! { cx, <p>"Should load after one second."</p>})}
            </Suspense>
            <p>"Should load along with the suspended content."</p>
        </main>
    }
}
