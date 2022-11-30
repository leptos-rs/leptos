use std::time::Duration;

use gloo_timers::future::TimeoutFuture;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    url: String,
}

async fn fetch_cats(count: u32) -> Result<Vec<String>, ()> {
    // artificial delay
    // the cat API is too fast to show the transition
    TimeoutFuture::new(500).await;

    if count > 0 {
        let res = reqwasm::http::Request::get(&format!(
            "https://api.thecatapi.com/v1/images/search?limit={}",
            count
        ))
        .send()
        .await
        .map_err(|_| ())?
        .json::<Vec<Cat>>()
        .await
        .map_err(|_| ())?
        .into_iter()
        .map(|cat| cat.url)
        .collect::<Vec<_>>();
        Ok(res)
    } else {
        Ok(vec![])
    }
}

pub fn fetch_example(cx: Scope) -> web_sys::Element {
    let (cat_count, set_cat_count) = create_signal::<u32>(cx, 1);
    let cats = create_resource(cx, cat_count, |count| fetch_cats(count));
    let (pending, set_pending) = create_signal(cx, false);

    view! { cx,
        <div>
            <label>
                "How many cats would you like?"
                <input type="number"
                    value={move || cat_count.get().to_string()}
                    on:input=move |ev| {
                        let val = event_target_value(&ev).parse::<u32>().unwrap_or(0);
                        set_cat_count(val);
                    }
                />
            </label>
            {move || pending().then(|| view! { cx, <p>"Loading more cats..."</p> })}
            <div>
                // <Transition/> holds the previous value while new async data is being loaded
                // Switch the <Transition/> to <Suspense/> to fall back to "Loading..." every time
                <Transition
                    fallback={"Loading (Suspense Fallback)...".to_string()}
                    set_pending
                >
                    {move || {
                            cats.read().map(|data| match data {
                                Err(_) => view! { cx,  <pre>"Error"</pre> },
                                Ok(cats) => view! { cx,
                                    <div>{
                                        cats.iter()
                                            .map(|src| {
                                                view! { cx,
                                                    <img src={src}/>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    }</div>
                                },
                            })
                        }
                    }
                </Transition>
            </div>
        </div>
    }
}
