use leptos::prelude::*;
use leptos_router::{components::Route, path, MatchNestedRoutes};
#[cfg(feature = "ssr")]
use std::time::Duration;

// https://github.com/leptos-rs/leptos/issues/4897

#[component]
pub fn Routes4897() -> impl MatchNestedRoutes + Clone {
    view! { <Route path=path!("4897") view=Page4897/> }.into_inner()
}

#[component]
fn Page4897() -> impl IntoView {
    view! {
        <p id="index-marker">"Issue 4897"</p>
        <OuterError4897/>
        <Outer4897 id="slow" n=4 delay_ms=50/>
        <Outer4897 id="fast" n=8 delay_ms=1/>
    }
}

#[component]
fn OuterError4897() -> impl IntoView {
    let outer = OnceResource::new(delay_4897(0, 100));

    view! {
        <Suspense fallback=|| "loading outer">
            {move || Suspend::new(async move {
                outer.await.map(|_| view! { <InnerError4897/> })
            })}
        </Suspense>
    }
}

#[component]
fn InnerError4897() -> impl IntoView {
    let inner = OnceResource::new(fail_4897());
    let (clicks, set_clicks) = signal(0);

    view! {
        <Suspense fallback=|| "loading inner">
            <ErrorBoundary fallback=move |errors| view! {
                <p id="error-count">{move || errors.with(|errors| errors.iter().count())}</p>
                <button id="error-bump" on:click=move |_| *set_clicks.write() += 1>
                    "bump"
                </button>
                <p id="error-clicks">{clicks}</p>
            }>
                {move || Suspend::new(async move { inner.await })}
            </ErrorBoundary>
        </Suspense>
    }
}

#[server]
async fn fail_4897() -> Result<i64, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(1)).await;
    Err(ServerFnError::new("failed"))
}

#[component]
fn Outer4897(id: &'static str, n: i64, delay_ms: u64) -> impl IntoView {
    let outer = OnceResource::new(delay_4897(n, delay_ms));

    view! {
        <Suspense fallback=|| "loading outer">
            {move || Suspend::new(async move {
                outer.await.map(|n| view! { <Inner4897 id n/> })
            })}
        </Suspense>
    }
}

#[component]
fn Inner4897(id: &'static str, n: i64) -> impl IntoView {
    let inner = OnceResource::new(delay_4897(n + 1, 1));
    let (count, set_count) = signal(0);

    view! {
        <Suspense fallback=|| "loading inner">
            {move || Suspend::new(async move {
                inner.await.map(|value| view! {
                    <p id=format!("{id}-value")>{move || value + count.get()}</p>
                    <button
                        id=format!("{id}-bump")
                        on:click=move |_| *set_count.write() += 1
                    >
                        "bump"
                    </button>
                })
            })}
        </Suspense>
    }
}

#[server]
async fn delay_4897(n: i64, delay_ms: u64) -> Result<i64, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    Ok(n)
}
