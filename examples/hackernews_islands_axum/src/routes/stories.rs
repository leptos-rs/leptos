use crate::api;
use leptos::{either::Either, prelude::*};
use leptos_router::{
    components::A,
    hooks::{use_params_map, use_query_map},
};

fn category(from: &str) -> String {
    match from {
        "new" => "newest",
        "show" => "show",
        "ask" => "ask",
        "job" => "jobs",
        _ => "news",
    }
    .to_string()
}

#[server]
pub async fn fetch_stories(
    story_type: String,
    page: usize,
) -> Result<Vec<api::Story>, ServerFnError> {
    let path = format!("{}?page={}", category(&story_type), page);
    Ok(api::fetch_api::<Vec<api::Story>>(&api::story(&path))
        .await
        .unwrap_or_default())
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
            .unwrap_or_else(|| "top".to_string())
    };
    let stories = Resource::new(
        move || (page(), story_type()),
        move |(page, story_type)| async move {
            fetch_stories(story_type, page).await.ok()
        },
    );
    let (pending, set_pending) = signal(false);

    let hide_more_link = move || match &*stories.read() {
        Some(Some(stories)) => stories.len() < 28,
        _ => true
    } || pending.get();

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
                        class:disabled=hide_more_link
                        aria-hidden=hide_more_link
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
                        set_pending
                    >
                        <Show when=move || stories.read().as_ref().map(Option::is_none).unwrap_or(false)>
                        >
                            <p>"Error loading stories."</p>
                        </Show>
                        <ul>
                            <For
                                each=move || stories.get().unwrap_or_default().unwrap_or_default()
                                key=|story| story.id
                                let:story
                            >
                                <Story story/>
                            </For>
                        </ul>
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
