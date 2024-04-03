use crate::api;
use leptos::*;
use leptos_router::*;

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
            .with(|q| q.get("page").and_then(|page| page.parse::<usize>().ok()))
            .unwrap_or(1)
    };
    let story_type = move || {
        params
            .with(|p| p.get("stories").cloned())
            .unwrap_or_else(|| "top".to_string())
    };
    let stories = create_resource(
        move || (page(), story_type()),
        move |(page, story_type)| async move {
            let path = format!("{}?page={}", category(&story_type), page);
            api::fetch_api::<Vec<api::Story>>(&api::story(&path)).await
        },
    );
    let (pending, set_pending) = create_signal(false);

    let hide_more_link = move || {
        stories.get().unwrap_or(None).unwrap_or_default().len() < 28
            || pending.get()
    };

    view! {
        <div class="news-view">
            <div class="news-list-nav">
                <span>
                    {move || if page() > 1 {
                        view! {

                            <a class="page-link"
                                href=move || format!("/{}?page={}", story_type(), page() - 1)
                                attr:aria_label="Previous Page"
                            >
                                "< prev"
                            </a>
                        }.into_any()
                    } else {
                        view! {

                            <span class="page-link disabled" aria-hidden="true">
                                "< prev"
                            </span>
                        }.into_any()
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
                    <Transition
                        fallback=move || view! { <p>"Loading..."</p> }
                        set_pending
                    >
                        {move || match stories.get() {
                            None => None,
                            Some(None) => Some(view! { <p>"Error loading stories."</p> }.into_any()),
                            Some(Some(stories)) => {
                                Some(view! {
                                    <ul>
                                        <For
                                            each=move || stories.clone()
                                            key=|story| story.id
                                            let:story
                                        >
                                            <Story story/>
                                        </For>
                                    </ul>
                                }.into_any())
                            }
                        }}
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
                    view! {
                        <span>
                            <a href=story.url target="_blank" rel="noreferrer">
                                {story.title.clone()}
                            </a>
                            <span class="host">"("{story.domain}")"</span>
                        </span>
                    }.into_view()
                } else {
                    let title = story.title.clone();
                    view! { <A href=format!("/stories/{}", story.id)>{title.clone()}</A> }.into_view()
                }}
            </span>
            <br />
            <span class="meta">
                {if story.story_type != "job" {
                    view! {
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
                    }.into_view()
                } else {
                    let title = story.title.clone();
                    view! { <A href=format!("/item/{}", story.id)>{title.clone()}</A> }.into_view()
                }}
            </span>
            {(story.story_type != "link").then(|| view! {
                " "
                <span class="label">{story.story_type}</span>
            })}
        </li>
    }
}
