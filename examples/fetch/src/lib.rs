use leptos::prelude::*;
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

async fn fetch_cats(count: CatCount) -> Result<Vec<String>, Error> {
    if count > 0 {
        gloo_timers::future::TimeoutFuture::new(1000).await;
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
    let (cat_count, set_cat_count) = signal::<CatCount>(1);

    let cats = LocalResource::new(move || fetch_cats(cat_count.get()));

    let fallback = move |errors: ArcRwSignal<Errors>| {
        let error_list = move || {
            errors.with(|errors| {
                errors
                    .iter()
                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                    .collect::<Vec<_>>()
            })
        };

        view! {
            <div class="error">
                <h2>"Error"</h2>
                <ul>{error_list}</ul>
            </div>
        }
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
            <Transition fallback=|| view! { <div>"Loading..."</div> }>
                <ErrorBoundary fallback>
                    <ul>
                        {move || Suspend::new(async move {
                            cats.await
                                .map(|cats| {
                                    cats.iter()
                                        .map(|s| {
                                            view! {
                                                <li>
                                                    <img src=s.clone() />
                                                </li>
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                })
                        })}

                    </ul>
                </ErrorBoundary>
            </Transition>
        </div>
    }
}
