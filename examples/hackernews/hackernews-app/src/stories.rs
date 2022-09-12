use std::time::Duration;

use leptos::*;

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

pub fn stories_data(cx: Scope, params: Memo<ParamsMap>, location: Location) -> StoriesData {
    log::debug!("(stories_data) loading data for stories");
    let page = create_memo(cx, move |_| {
        location
            .query
            .with(|q| {
                log::debug!("(stories_data) q.page == {:?}", q.get("page"));
                q.get("page").and_then(|p| p.parse::<usize>().ok())
            })
            .unwrap_or(1)
    });
    log::debug!("(stories_data) page == {}", page.get(),);
    let story_type = create_memo(cx, move |_| {
        params
            .with(|params| params.get("stories").cloned())
            .unwrap_or_else(|| "top".to_string())
    });
    let stories = create_resource(
        cx,
        move || format!("{}?page={}", category(&story_type()), page()),
        |path| async move {
            api::fetch_api::<Vec<api::Story>>(&api::story(&path))
                .await
                .map_err(|_| ())
        },
    );
    StoriesData {
        page,
        story_type,
        stories,
    }
}

#[derive(Clone)]
pub struct StoriesData {
    pub page: Memo<usize>,
    pub story_type: Memo<String>,
    pub stories: Resource<String, Result<Vec<api::Story>, ()>>,
}

impl std::fmt::Debug for StoriesData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoriesData").finish()
    }
}

#[component]
pub fn Stories(cx: Scope) -> Element {
    let StoriesData {
        page,
        story_type,
        stories,
    } = use_loader::<StoriesData>(cx);

    let hide_more_link = move || stories.read().unwrap_or(Err(())).unwrap_or_default().len() < 28;

    view! {
        <div class="news-view">
            <div class="news-list-nav">
                // TODO fix
                /* {move || if page() > 1 {
                    view! {
                        //<Link
                            //attr:class="page-link"
                            //to={format!("/{}?page={}", story_type(), page() - 1)}
                            //attr:aria_label="Previous Page"
                        <a href="#">//href={format!("/{}?page={}", story_type(), page() - 1)}
                            "< prev"
                        </a>//</Link>
                    }
                } else {
                    view! {
                        <span class="page-link disabled" aria-hidden="true">
                            "< prev"
                        </span>
                    }
                }} */
                <span>"page " {page}</span>
                <span class="page-link"
                    class:disabled={move || hide_more_link()}
                    aria-hidden={move || hide_more_link()}
                >
                    <a href={format!("/{}?page={}", story_type(), page() + 1)}
                        aria-label="Next Page"
                    >
                        "more >"
                    </a>
                </span>
            </div>
            <main class="news-list">
                <div>
                    <Suspense fallback=view! { <p>"Loading..."</p> }>
                        {move || match stories.read() {
                            None => None,
                            Some(Err(_)) => Some(view! { <p>"Error loading stories."</p> }),
                            Some(Ok(stories)) => {
                                Some(view! {
                                    <ul>
                                        <For each={move || stories.clone()} key=|story| story.id>{
                                            move |cx: Scope, story: &api::Story| {
                                                view! {
                                                    <Story story={story.clone()} />
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
    view! {
         <li class="news-item">
            <span class="score">{story.points}</span>
            <span class="title">
                {if !story.url.starts_with("item?id=") {
                    view! {
                        <span>
                            <a href={story.url} target="_blank" rel="noreferrer">
                                {story.title.clone()}
                            </a>
                            <span class="host">"("{story.domain}")"</span>
                        </span>
                    }
                } else {
                    let title = story.title.clone();
                    view! { <Link to={format!("/stories/{}", story.id)}>{title}</Link> }
                }}
            </span>
            <br />
            <span class="meta">
                {if story.story_type != "job" {
                    view! {
                        <span>
                            //{"by "}
                            //{story.user.map(|user| view ! { <Link to={format!("/users/{}", user)}>{&user}</Link>})}
                            //{format!(" {} | ", story.time_ago)}
                            <Link to={format!("/stories/{}", story.id)}>
                                {if story.comments_count.unwrap_or_default() > 0 {
                                    format!("{} comments", story.comments_count.unwrap_or_default())
                                } else {
                                    "discuss".into()
                                }}
                            </Link>
                        </span>
                    }
                } else {
                    let title = story.title.clone();
                    view! { <Link to={format!("/item/{}", story.id)}>{title}</Link> }
                }}
            </span>
            {(story.story_type != "link").then(|| view! {
                <span>
                    //{" "}
                    <span class="label">{story.story_type}</span>
                </span>
            })}
        </li>
    }
}
