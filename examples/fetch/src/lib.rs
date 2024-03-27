use leptos::{
    error::Result,
    prelude::*,
    reactive_graph::{
        computed::AsyncDerived,
        signal::{signal, RwSignal},
    },
    view, IntoView, Transition,
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
    let (cat_count, set_cat_count) = signal::<CatCount>(3);

    // we use new_unsync here because the reqwasm request type isn't Send
    // if we were doing SSR, then
    // 1) we'd want to use a Resource, so the data would be serialized to the client
    // 2) we'd need to make sure there was a thread-local spawner set up
    let cats = AsyncDerived::new_unsync(move || fetch_cats(*cat_count()));

    // TODO ErrorBoundary
    /*let fallback = move |errors: RwSignal<Errors>| {
        let error_list = move || {
            errors.with(|errors| {
                errors
                    .iter()
                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                    .collect_view()
            })
        };

        view! {
            <div class="error">
                <h2>"Error"</h2>
                <ul>{error_list}</ul>
            </div>
        }
    };*/

    view! {
        <div>
            <label>
                "How many cats would you like?"
                <input
                    type="number"
                    prop:value=move || cat_count.get().to_string()
                    on:input:target=move |ev| {
                        let val = ev.target().value().parse::<CatCount>().unwrap_or(0);
                        set_cat_count(val);
                    }
                />
            </label>
            <Transition fallback=|| view! { <div>"Loading..."</div> }>
                {async move {
                    cats.await
                        .map(|cats| {
                            cats.into_iter()
                                .map(|s| view! { <p><img src={s}/></p> })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                }}
            </Transition>
        </div>
    }
}
