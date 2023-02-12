use leptos::*;
use leptos_meta::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    let one_second = create_resource(
        cx,
        || (),
        |_| futures_timer::Delay::new(std::time::Duration::from_secs(1)),
    );

    let three_seconds = create_resource(
        cx,
        || (),
        |_| futures_timer::Delay::new(std::time::Duration::from_secs(2)),
    );

    view! { cx,
        <main>
            <h1>"Hello, world!"</h1>
            <p>"Some text"</p>
            <Suspense fallback=move || view! { cx, <p>"Loading..."</p> }>
                {move || one_second.read().map(|_| view! { cx,
                    <Meta name="title" content="Only works in async..."/>
                    <p>"Should load after one second."</p>
                    <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
                })}
            </Suspense>
            <p>"This either loads a) after the suspense (if in-order), b) before the suspense (if out-of-order), or c) with all the content (if async)"</p>
        </main>
    }
}
