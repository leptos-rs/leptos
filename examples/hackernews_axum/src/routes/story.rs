use crate::api::{self, Story};
use leptos::{either::Either, prelude::*};
use leptos_meta::Meta;
use leptos_router::{
    components::A, hooks::use_params_map, lazy_route, LazyRoute,
};

#[derive(Debug)]
pub struct StoryRoute {
    story: Resource<Option<Story>>,
}

#[lazy_route]
impl LazyRoute for StoryRoute {
    fn data() -> Self {
        let params = use_params_map();
        let story = Resource::new_blocking(
            move || params.read().get("id").unwrap_or_default(),
            move |id| async move {
                if id.is_empty() {
                    None
                } else {
                    api::fetch_api::<api::Story>(&api::story(&format!(
                        "item/{id}"
                    )))
                    .await
                }
            },
        );
        Self { story }
    }

    fn view(this: Self) -> AnyView {
        let StoryRoute { story } = this;
        Suspense(SuspenseProps::builder().fallback(|| "Loading...").children(ToChildren::to_children(move || Suspend::new(async move {
        match story.await.clone() {
            None => Either::Left("Story not found."),
            Some(story) => {
                Either::Right(view! {
                    <Meta name="description" content=story.title.clone()/>
                    <div class="item-view">
                        <div class="item-view-header">
                            <a href=story.url target="_blank">
                                <h1>{story.title}</h1>
                            </a>
                            <span class="host">
                                "("{story.domain}")"
                            </span>
                            <ShowLet some=story.user let:user>
                                <p class="meta">
                                    {story.points}
                                    " points | by "
                                    <A href=format!("/users/{user}")>{user.clone()}</A>
                                    {format!(" {}", story.time_ago)}
                                </p>
                            </ShowLet>
                        </div>
                        <div class="item-view-comments">
                            <p class="item-view-comments-header">
                                {if story.comments_count.unwrap_or_default() > 0 {
                                    format!("{} comments", story.comments_count.unwrap_or_default())
                                } else {
                                    "No comments yet.".into()
                                }}
                            </p>
                            <ul class="comment-children">
                                <For
                                    each=move || story.comments.clone().unwrap_or_default()
                                    key=|comment| comment.id
                                    let:comment
                                >
                                    <Comment comment />
                                </For>
                            </ul>
                        </div>
                    </div>
                })
            }
        }
    }))).build()).into_any()
    }
}

#[component]
pub fn Comment(comment: api::Comment) -> impl IntoView {
    let (open, set_open) = signal(true);

    view! {
        <li class="comment">
        <div class="by">
            <A href=format!("/users/{}", comment.user.clone().unwrap_or_default())>{comment.user.clone()}</A>
            {format!(" {}", comment.time_ago)}
        </div>
        <div class="text" inner_html=comment.content></div>
        {(!comment.comments.is_empty()).then(|| {
            view! {
                <div>
                    <div class="toggle" class:open=open>
                        <a on:click=move |_| set_open.update(|n| *n = !*n)>
                            {
                                let comments_len = comment.comments.len();
                                move || if open.get() {
                                    "[-]".into()
                                } else {
                                    format!("[+] {}{} collapsed", comments_len, pluralize(comments_len))
                                }
                            }
                        </a>
                    </div>
                    {move || open.get().then({
                        let comments = comment.comments.clone();
                        move || view! {
                            <ul class="comment-children">
                                <For
                                    each=move || comments.clone()
                                    key=|comment| comment.id
                                    let:comment
                                >
                                    <Comment comment />
                                </For>
                            </ul>
                        }
                    })}
                </div>
            }
        })}
        </li>
    }.into_any()
}

fn pluralize(n: usize) -> &'static str {
    if n == 1 {
        " reply"
    } else {
        " replies"
    }
}
