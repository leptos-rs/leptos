use std::sync::LazyLock;

use leptos::prelude::*;
use leptos_meta::MetaTags;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, ProtectedRoute, Route, Router},
    hooks::use_params,
    params::Params,
    ParamSegment, SsrMode, StaticSegment,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let count = RwSignal::new(2);
    let posts = Resource::new(
        move || count.get(),
        |count| async move {
            if count % 2 == 0 {
                (0..count).collect::<Vec<_>>()
            } else {
                vec![]
            }
        },
    );

    view! {
        <button on:click=move |_| *count.write() += 1>"+1"</button>
            <p>
        <Suspense fallback=|| "Loading...">
            {move || Suspend::new(async move {
                let posts = posts.await;
                posts
                    .into_iter()
                    .map(|post| {
                        view! {
                            <div>{post}</div>
                        }
                    })
                    .collect_view()
            })}
        </Suspense>
            </p>
    }
}
