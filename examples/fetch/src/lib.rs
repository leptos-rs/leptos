use anyhow::Result;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    url: String,
}

async fn fetch_cats(count: u32) -> Result<Vec<String>> {
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
        .map(|cat| cat.url)
        .collect::<Vec<_>>();
        Ok(res)
    } else {
        Ok(vec![])
    }
}

pub fn fetch_example(cx: Scope) -> impl IntoView {
    let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);

    // we use local_resource here because
    // 1) anyhow::Result isn't serializable/deserializable
    // 2) we're not doing server-side rendering in this example anyway
    //    (during SSR, create_resource will begin loading on the server and resolve on the client)
    let cats = create_local_resource(cx, cat_count, fetch_cats);

    let fallback = move |cx, errors: RwSignal<Errors>| {
        let error_list = move || {
            errors.with(|errors| {
                errors
                    .iter()
                    .map(|(_, e)| view! { cx, <li>{e.to_string()}</li>})
                    .collect::<Vec<_>>()
            })
        };

        view! { cx,
            <div class="error">
                <h2>"Error"</h2>
                <ul>{error_list}</ul>
            </div>
        }
    };

    // the renderer can handle Option<_> and Result<_> states
    // by displaying nothing for None if the resource is still loading
    // and by using the ErrorBoundary fallback to catch Err(_)
    // so we'll just implement our happy path and let the framework handle the rest
    let cats_view = move || {
        cats.with(cx, |data| {
            data.iter()
                .flatten()
                .map(|cat| view! { cx, <img src={cat}/> })
                .collect::<Vec<_>>()
        })
    };

    view! { cx,
        <div>
            <label>
                "How many cats would you like?"
                <input type="number"
                    prop:value={move || cat_count.get().to_string()}
                    on:input=move |ev| {
                        let val = event_target_value(&ev).parse::<u32>().unwrap_or(0);
                        set_cat_count(val);
                    }
                />
            </label>
            <ErrorBoundary fallback>
                <Transition fallback=move || view! { cx, <div>"Loading (Suspense Fallback)..."</div>}>
                    {cats_view}
                </Transition>
            </ErrorBoundary>
        </div>
    }
}
