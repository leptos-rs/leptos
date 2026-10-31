use leptos::prelude::*;
use leptos_router::{components::Route, path, MatchNestedRoutes};
#[cfg(feature = "ssr")]
use std::time::Duration;

// https://github.com/leptos-rs/leptos/issues/4980

#[component]
pub fn Routes4980() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4980") view=Page4980/>
    }
    .into_inner()
}

#[component]
fn Page4980() -> impl IntoView {
    let first = Resource::new(|| (), |_| data_4980(4));
    let second = Resource::new(|| (), |_| data_4980(5));

    view! {
        <div id="await">
            <ErrorBoundary fallback=|_| ()>
                <Await future=data_4980(3) let:nums>
                    <Counter4980 text=nums.clone().map(|v| v.join(",")).unwrap_or_default()/>
                </Await>
            </ErrorBoundary>
        </div>
        <div id="first">
            <Suspense>{move || first.and_then(|v| v.join(","))}</Suspense>
        </div>
        <div id="second">
            <Suspense>{move || second.and_then(|v| v.join(","))}</Suspense>
        </div>
    }
}

#[component]
fn Counter4980(text: String) -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <p id="await-items">{text}</p>
        <button id="await-bump" on:click=move |_| *set_count.write() += 1>"bump"</button>
        <p id="await-count">{count}</p>
    }
}

#[server]
async fn data_4980(n: usize) -> Result<Vec<String>, ServerFnError> {
    if n > 3 {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok((1..=n).map(|n| n.to_string()).collect())
}
