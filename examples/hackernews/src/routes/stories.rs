use leptos::*;
use leptos_router::*;

use crate::api;

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
pub fn Stories(cx: Scope) -> Element {
    let query = use_query_map(cx);
    let params = use_params_map(cx);
    let page = move || {
        query
            .with(|q| q.get("page").and_then(|page| page.parse::<usize>().ok()))
            .unwrap_or(1)
    };
    let story_type = move || {
        params
            .with(|p| p.get("stories").cloned())
            .unwrap_or_else(|| "top".to_string())
    };
    let stories = create_resource(
        cx,
        move || (page(), story_type()),
        move |(page, story_type)| async move {
            let path = format!("{}?page={}", category(&story_type), page);
            api::fetch_api::<Vec<api::Story>>(cx, &api::story(&path)).await
        },
    );

    let hide_more_link = move || stories.read().unwrap_or(None).unwrap_or_default().len() < 28;

    view! {
        cx,
        <div class="news-view">
            <div class="news-list-nav">
                <span>
                    {move || if page() > 1 {
                        view! {
                            cx,
                            <a class="page-link"
                                href=move || format!("/{}?page={}", story_type(), page() - 1)
                                attr:aria_label="Previous Page"
                            >
                                "< prev"
                            </a>
                        }
                    } else {
                        view! {
                            cx,
                            <span class="page-link disabled" aria-hidden="true">
                                "< prev"
                            </span>
                        }
                    }}
                </span>
                <span>"page " {page}</span>
                <span class="page-link"
                    class:disabled=hide_more_link
                    aria-hidden=hide_more_link
                >
                    <a href=move || format!("/{}?page={}", story_type(), page() + 1)
                        aria-label="Next Page"
                    >
                        "more >"
                    </a>
                </span>
            </div>
            <main class="news-list">
                <div>
                    <Suspense fallback=view! { cx,  <p>"Loading..."</p> }>
                        {move || match stories.read() {
                            None => None,
                            Some(None) => Some(view! { cx,  <p>"Error loading stories."</p> }),
                            Some(Some(stories)) => {
                                Some(view! { cx,
                                    <ul>
                                        <For each=move || stories.clone() key=|story| story.id>{
                                            move |cx: Scope, story: &api::Story| {
                                                view! { cx,
                                                    <Story story=story.clone() />
                                                }
                                            }
                                        }</For>
                                    </ul>
                                })
                            }
                        }}
                    </Suspense>
                </div>
            </main>
        </div>
    }
}

#[component]
fn Story(cx: Scope, story: api::Story) -> Element {
    view! { cx,
         <li class="news-item">
            <span class="score">{story.points}</span>
            <span class="title">
                {if !story.url.starts_with("item?id=") {
                    view! { cx,
                        <span>
                            <a href=story.url target="_blank" rel="noreferrer">
                                {story.title.clone()}
                            </a>
                            <span class="host">"("{story.domain}")"</span>
                        </span>
                    }
                } else {
                    let title = story.title.clone();
                    view! { cx,  <A href=format!("/stories/{}", story.id)>{title.clone()}</A> }
                }}
            </span>
            <br />
            <span class="meta">
                {if story.story_type != "job" {
                    view! { cx,
                        <span>
                            {"by "}
                            {story.user.map(|user| view ! { cx, <A href=format!("/users/{}", user)>{user.clone()}</A>})}
                            {format!(" {} | ", story.time_ago)}
                            <A href=format!("/stories/{}", story.id)>
                                {if story.comments_count.unwrap_or_default() > 0 {
                                    format!("{} comments", story.comments_count.unwrap_or_default())
                                } else {
                                    "discuss".into()
                                }}
                            </A>
                        </span>
                    }
                } else {
                    let title = story.title.clone();
                    view! { cx,  <A href=format!("/item/{}", story.id)>{title.clone()}</A> }
                }}
            </span>
            {(story.story_type != "link").then(|| view! { cx,
                <span>
                    //{" "}
                    <span class="label">{story.story_type}</span>
                </span>
            })}
        </li>
    }
}
