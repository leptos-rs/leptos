use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    path, MatchNestedRoutes, SsrMode,
};
#[cfg(feature = "ssr")]
use std::time::Duration;

// https://github.com/leptos-rs/leptos/issues/4908

#[component]
pub fn Routes4908() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4908") view=Outlet>
            <Route path=path!("") view=Page4908/>
            <Route path=path!("in-order") view=Page4908 ssr=SsrMode::InOrder/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Page4908() -> impl IntoView {
    // each case sits next to sibling <Suspense/>s, so that content being hydrated into the
    // wrong boundary would show up as a mismatch in one of the cases
    view! {
        <p id="index-marker">"Issue 4908"</p>
        <ErrorBoundaryOuter4908 id="eb-suspense-ok" fail=false/>
        <ErrorBoundaryOuter4908 id="eb-suspense-err" fail=true/>
        <SuspenseOuter4908 id="suspense-eb-ok" fail=false/>
        <SuspenseOuter4908 id="suspense-eb-err" fail=true/>
    }
}

#[component]
fn ErrorBoundaryOuter4908(id: &'static str, fail: bool) -> impl IntoView {
    let data = Resource::new(|| (), move |_| data_4908(fail));

    view! {
        <ErrorBoundary fallback=move |errors| view! { <Fallback4908 id errors/> }>
            <Suspense fallback=|| "Loading...">
                {move || data.and_then(|items| view! { <Items4908 id items=items.clone()/> })}
            </Suspense>
        </ErrorBoundary>
    }
}

#[component]
fn SuspenseOuter4908(id: &'static str, fail: bool) -> impl IntoView {
    let data = Resource::new(|| (), move |_| data_4908(fail));

    view! {
        <Suspense fallback=|| "Loading...">
            <ErrorBoundary fallback=move |errors| view! { <Fallback4908 id errors/> }>
                {move || data.and_then(|items| view! { <Items4908 id items=items.clone()/> })}
            </ErrorBoundary>
        </Suspense>
    }
}

#[component]
fn Items4908(id: &'static str, items: Vec<String>) -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <p id=format!("{id}-items")>{items.join(",")}</p>
        <button id=format!("{id}-bump") on:click=move |_| *set_count.write() += 1>
            "bump"
        </button>
        <p id=format!("{id}-count")>{count}</p>
    }
}

#[component]
fn Fallback4908(id: &'static str, errors: ArcRwSignal<Errors>) -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <p id=format!("{id}-error-count")>
            {move || errors.with(|errors| errors.iter().count())}
        </p>
        <button id=format!("{id}-error-bump") on:click=move |_| *set_count.write() += 1>
            "bump"
        </button>
        <p id=format!("{id}-error-clicks")>{count}</p>
    }
}

#[server]
async fn data_4908(fail: bool) -> Result<Vec<String>, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(50)).await;
    if fail {
        Err(ServerFnError::new("failed"))
    } else {
        Ok(vec!["1".into(), "2".into(), "3".into()])
    }
}
