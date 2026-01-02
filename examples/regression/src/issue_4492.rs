use leptos::prelude::*;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, MatchNestedRoutes, NavigateOptions,
};

#[component]
pub fn Routes4492() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4492") view=Issue4492/>
    }
    .into_inner()
}

#[component]
fn Issue4492() -> impl IntoView {
    let show_a = RwSignal::new(false);
    let show_b = RwSignal::new(false);
    let show_c = RwSignal::new(false);

    view! {
        <button id="a-toggle" on:click=move |_| show_a.set(!show_a.get())>"Toggle A"</button>
        <button id="b-toggle" on:click=move |_| show_b.set(!show_b.get())>"Toggle B"</button>
        <button id="c-toggle" on:click=move |_| show_c.set(!show_c.get())>"Toggle C"</button>

        <Show when=move || show_a.get()>
            <ScenarioA/>
        </Show>
        <Show when=move || show_b.get()>
            <ScenarioB/>
        </Show>
        <Show when=move || show_c.get()>
            <ScenarioC/>
        </Show>
    }
}

#[component]
fn ScenarioA() -> impl IntoView {
    // scenario A: one truly-async resource is read on click
    let counter = RwSignal::new(0);
    let resource = Resource::new(
        move || counter.get(),
        |count| async move {
            sleep(50).await.unwrap();
            count
        },
    );
    view! {
        <Transition fallback=|| view! { <p id="a-result">"Loading..."</p> }>
            <p id="a-result">{resource}</p>
        </Transition>
        <button id="a-button" on:click=move |_| *counter.write() += 1>"+1"</button>
    }
}

#[component]
fn ScenarioB() -> impl IntoView {
    // scenario B: resource immediately available first time, then after 250ms
    let counter = RwSignal::new(0);
    let resource = Resource::new(
        move || counter.get(),
        |count| async move {
            if count == 0 {
                count
            } else {
                sleep(50).await.unwrap();
                count
            }
        },
    );
    view! {
        <Transition fallback=|| view! { <p id="b-result">"Loading..."</p> }>
            <p id="b-result">{resource}</p>
        </Transition>
        <button id="b-button" on:click=move |_| *counter.write() += 1>"+1"</button>
    }
}

#[component]
fn ScenarioC() -> impl IntoView {
    // scenario C: not even a resource on the first run, just a value
    // see https://github.com/leptos-rs/leptos/issues/3868
    let counter = RwSignal::new(0);
    let s_res = StoredValue::new(None::<ArcLocalResource<i32>>);
    let resource = move || {
        let count = counter.get();
        if count == 0 {
            count
        } else {
            let r = s_res.get_value().unwrap_or_else(|| {
                let res = ArcLocalResource::new(move || async move {
                    sleep(50).await.unwrap();
                    count
                });
                s_res.set_value(Some(res.clone()));
                res
            });
            r.get().unwrap_or(42)
        }
    };
    view! {
        <Transition fallback=|| view! { <p id="c-result">"Loading..."</p> }>
            <p id="c-result">{resource}</p>
        </Transition>
        <button id="c-button" on:click=move |_| *counter.write() += 1>"+1"</button>
    }
}

#[server]
async fn sleep(ms: u64) -> Result<(), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    Ok(())
}
