use leptos::{
    error::Result,
    prelude::*,
    reactive_graph::{
        computed::AsyncDerived,
        signal::{signal, RwSignal},
    },
    view, ErrorBoundary, IntoView, Transition,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    url: String,
}

#[derive(Error, Clone, Debug)]
pub enum CatError {
    #[error("Please request more than zero cats.")]
    NonZeroCats,
}

type CatCount = usize;

async fn fetch_cats(count: CatCount) -> Result<Vec<String>> {
    if count > 0 {
        // make the request
        let res = reqwasm::http::Request::get(&format!(
            "https://api.thecatapi.com/v1/images/search?limit={count}",
        ))
        .send()
        .await?
        // convert it to JSON
        .json::<Vec<Cat>>()
        .await?
        // extract the URL field for each cat
        .into_iter()
        .take(count)
        .map(|cat| cat.url)
        .collect::<Vec<_>>();
        Ok(res)
    } else {
        Err(CatError::NonZeroCats)?
    }
}

pub fn fetch_example() -> impl IntoView {
    let (cat_count, set_cat_count) = signal::<CatCount>(0);

    // we use new_unsync here because the reqwasm request type isn't Send
    // if we were doing SSR, then
    // 1) we'd want to use a Resource, so the data would be serialized to the client
    // 2) we'd need to make sure there was a thread-local spawner set up
    let cats = AsyncDerived::new_unsync(move || fetch_cats(cat_count.get()));

    // TODO weaving together Transition and ErrorBoundary is hard with the new async API for
    // suspense, because Transition expects a Future as its children, and ErrorBoundary isn't a
    // future
    let cats_view = move || async move {
        cats.await.map(|cats| {
            cats.into_iter()
                .map(|s| view! { <p><img src={s}/></p> })
                .collect::<Vec<_>>()
        })
        //.catch(|e| view! { <p class="error">{e.to_string()}</p> })
    };

    view! {
        <div>
            <label>
                "How many cats would you like?"
                <input
                    type="number"
                    prop:value=move || cat_count.get().to_string()
                    on:input:target=move |ev| {
                        let val = ev.target().value().parse::<CatCount>().unwrap_or(0);
                        set_cat_count.set(val);
                    }
                />
            </label>
                <ErrorBoundary fallback=|e| view! { <p class="error">{e.to_string()}</p> }>
                    <Transition fallback=|| view! { <div>"Loading..."</div> }>
                        {cats_view()}
                    </Transition>
                </ErrorBoundary>
        </div>
    }
}
