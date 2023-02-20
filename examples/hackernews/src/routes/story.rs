use crate::api;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn Story(cx: Scope) -> impl IntoView {
    let params = use_params_map(cx);
    let story = create_resource(
        cx,
        move || params().get("id").cloned().unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                None
            } else {
                api::fetch_api::<api::Story>(cx, &api::story(&format!("item/{id}"))).await
            }
        },
    );
    let meta_description = move || story.read(cx).and_then(|story| story.map(|story| story.title)).unwrap_or_else(|| "Loading story...".to_string());

    view! { cx,
        <>
            <Meta name="description" content=meta_description/>
                <Suspense fallback=|| view! { cx, "Loading..." }>
                    {move || story.read(cx).map(|story| match story {
                        None => view! { cx,  <div class="item-view">"Error loading this story."</div> },
                        Some(story) => view! { cx,
                            <div class="item-view">
                                <div class="item-view-header">
                                <a href=story.url target="_blank">
                                    <h1>{story.title}</h1>
                                </a>
                                <span class="host">
                                    "("{story.domain}")"
                                </span>
                                {story.user.map(|user| view! { cx,  <p class="meta">
                                    {story.points}
                                    " points | by "
                                    <A href=format!("/users/{user}")>{user.clone()}</A>
                                    {format!(" {}", story.time_ago)}
                                </p>})}
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
                                        view=move |cx, comment| view! { cx,  <Comment comment /> }
                                    />
                                </ul>
                            </div>
                        </div>
                    }})
                }
            </Suspense>
        </>
    }
}

#[component]
pub fn Comment(cx: Scope, comment: api::Comment) -> impl IntoView {
    let (open, set_open) = create_signal(cx, true);

    view! { cx,
        <li class="comment">
        <div class="by">
            <A href=format!("/users/{}", comment.user.clone().unwrap_or_default())>{comment.user.clone()}</A>
            {format!(" {}", comment.time_ago)}
        </div>
        <div class="text" inner_html=comment.content></div>
        {(!comment.comments.is_empty()).then(|| {
            view! { cx,
                <div>
                    <div class="toggle" class:open=open>
                        <a on:click=move |_| set_open.update(|n| *n = !*n)>
                            {
                                let comments_len = comment.comments.len();
                                move || if open() {
                                    "[-]".into()
                                } else {
                                    format!("[+] {}{} collapsed", comments_len, pluralize(comments_len))
                                }
                            }
                        </a>
                    </div>
                    {move || open().then({
                        let comments = comment.comments.clone();
                        move || view! { cx,
                            <ul class="comment-children">
                                <For
                                    each=move || comments.clone()
                                    key=|comment| comment.id
                                    view=move |cx, comment: api::Comment| view! { cx, <Comment comment /> }
                                />
                            </ul>
                        }
                    })}
                </div>
            }
        })}
        </li>
    }
}

fn pluralize(n: usize) -> &'static str {
    if n == 1 {
        " reply"
    } else {
        " replies"
    }
}
