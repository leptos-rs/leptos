use crate::api;
use leptos::{either::Either, prelude::*};
use leptos_router::{
    components::A,
    hooks::{use_params_map, use_query_map},
};

fn category(from: &str) -> &'static str {
    match from {
        "new" => "newest",
        "show" => "show",
        "ask" => "ask",
        "job" => "jobs",
        _ => "news",
    }
}

#[component]
pub fn Stories() -> impl IntoView {
    let query = use_query_map();
    let params = use_params_map();
    let page = move || {
        query
            .read()
            .get("page")
            .and_then(|page| page.parse::<usize>().ok())
            .unwrap_or(1)
    };
    let story_type = move || {
        params
            .read()
            .get("stories")
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "top".to_string())
    };
    let stories = Resource::new_serde(
        move || (page(), story_type()),
        move |(page, story_type)| async move {
            let path = format!("{}?page={}", category(&story_type), page);
            api::fetch_api::<Vec<api::Story>>(&api::story(&path)).await
        },
    );
    let (pending, set_pending) = create_signal(false);

    let hide_more_link = move || {
        Suspend(async move {
            stories.await.unwrap_or_default().len() < 28 || pending.get()
        })
    };

    view! {
        <div class="news-view">
            <div class="news-list-nav">
                <span>
                    {move || if page() > 1 {
                        Either::Left(view! {
                            <a class="page-link"
                                href=move || format!("/{}?page={}", story_type(), page() - 1)
                                aria-label="Previous Page"
                            >
                                "< prev"
                            </a>
                        })
                    } else {
                        Either::Right(view! {
                            <span class="page-link disabled" aria-hidden="true">
                                "< prev"
                            </span>
                        })
                    }}
                </span>
                <span>"page " {page}</span>
                <Suspense>
                    <span class="page-link"
                        // TODO support Suspense in attributes
                        /*class:disabled=Suspend(hide_more_link)
                        aria-hidden=Suspend(hide_more_link)*/
                    >
                        <a href=move || format!("/{}?page={}", story_type(), page() + 1)
                            aria-label="Next Page"
                        >
                            "more >"
                        </a>
                    </span>
                </Suspense>
            </div>
            <main class="news-list">
                <div>
                    <Transition
                        fallback=move || view! { <p>"Loading..."</p> }
                        // TODO set_pending on Transition
                        //set_pending
                    >
                        {move || Suspend(async move { match stories.await {
                            None => Either::Left(view! { <p>"Error loading stories."</p> }),
                            Some(stories) => {
                                Either::Right(view! {
                                    <ul>
                                    {stories.into_iter().map(|story| view! { <Story story/> }).collect::<Vec<_>>()}
                                    </ul>
                                })
                            }
                        }})}
                    </Transition>
                </div>
            </main>
        </div>
    }
}

#[component]
fn Story(story: api::Story) -> impl IntoView {
    view! {
         <li class="news-item">
            <span class="score">{story.points}</span>
            <span class="title">
                {if !story.url.starts_with("item?id=") {
                    Either::Left(view! {
                        <span>
                            <a href=story.url target="_blank" rel="noreferrer">
                                {story.title.clone()}
                            </a>
                            <span class="host">"("{story.domain}")"</span>
                        </span>
                    })
                } else {
                    let title = story.title.clone();
                    Either::Right(view! { <A href=format!("/stories/{}", story.id)>{title}</A> })
                }}
            </span>
            <br />
            <span class="meta">
                {if story.story_type != "job" {
                    Either::Left(view! {
                        <span>
                            {"by "}
                            {story.user.map(|user| view ! {  <A href=format!("/users/{user}")>{user.clone()}</A>})}
                            {format!(" {} | ", story.time_ago)}
                            <A href=format!("/stories/{}", story.id)>
                                {if story.comments_count.unwrap_or_default() > 0 {
                                    format!("{} comments", story.comments_count.unwrap_or_default())
                                } else {
                                    "discuss".into()
                                }}
                            </A>
                        </span>
                    })
                } else {
                    let title = story.title.clone();
                    Either::Right(view! { <A href=format!("/item/{}", story.id)>{title}</A> })
                }}
            </span>
            {(story.story_type != "link").then(|| view! {
                " "
                <span class="label">{story.story_type}</span>
            })}
        </li>
    }
}
