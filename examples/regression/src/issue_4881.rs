use leptos::prelude::*;
use leptos_router::{components::Route, path, MatchNestedRoutes};

// https://github.com/leptos-rs/leptos/issues/4881
#[component]
pub fn Routes4881() -> impl MatchNestedRoutes + Clone {
    view! { <Route path=path!("4881") view=Page4881/> }.into_inner()
}

#[component]
fn Page4881() -> impl IntoView {
    view! {
        <p id="index-marker">"Issue 4881"</p>
        <div id="cards">
            <For each=|| [1u32, 2, 3] key=|item| *item let:item>
                <Card4881 item/>
            </For>
        </div>
    }
}

#[component]
fn Card4881(item: u32) -> impl IntoView {
    let files = Resource::new(move || item, list_files_4881);
    let (count, set_count) = signal(0);

    view! {
        <section>
            <h2>{format!("item {item}")}</h2>
            {move || {
                view! {
                    <Suspense fallback=|| "loading files">
                        {move || Suspend::new(async move {
                            let names = files.await.unwrap_or_default();
                            let joined = names.join(", ");
                            view! {
                                <ul>
                                    <For each=move || names.clone() key=|n| n.clone() let:name>
                                        <li>{name}</li>
                                    </For>
                                </ul>
                                <p id=format!("files-{item}")>{joined}</p>
                                <button
                                    id=format!("bump-{item}")
                                    on:click=move |_| *set_count.write() += 1
                                >
                                    "bump"
                                </button>
                                <p id=format!("count-{item}")>{count}</p>
                            }
                        })}
                    </Suspense>
                }
            }}
        </section>
    }
}

#[server]
async fn list_files_4881(item: u32) -> Result<Vec<String>, ServerFnError> {
    Ok((0..item).map(|i| format!("item {item} file {i}")).collect())
}
