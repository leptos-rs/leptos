use crate::api;
use leptos::{either::Either, prelude::*};
use leptos_meta::Meta;
use leptos_router::{components::A, hooks::use_params_map};

#[server]
pub async fn fetch_story(
    id: String,
) -> Result<Option<api::Story>, ServerFnError> {
    Ok(api::fetch_api::<api::Story>(&api::story(&format!("item/{id}"))).await)
}

#[component]
pub fn Story() -> impl IntoView {
    let params = use_params_map();
    let story = Resource::new_blocking(
        move || params.read().get("id").unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                Ok(None)
            } else {
                fetch_story(id).await
            }
        },
    );

    Suspense(SuspenseProps::builder().fallback(|| "Loading...").children(ToChildren::to_children(move || Suspend::new(async move {
        match story.await.ok().flatten() {
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
                        {story.user.map(|user| view! {  <p class="meta">
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
    }))).build())
}

#[component]
pub fn Comment(comment: api::Comment) -> impl IntoView {
    view! {
        <li class="comment">
            <div class="by">
                <A href=format!("/users/{}", comment.user.clone().unwrap_or_default())>{comment.user.clone()}</A>
                {format!(" {}", comment.time_ago)}
            </div>
            <div class="text" inner_html=comment.content></div>
            {(!comment.comments.is_empty()).then(|| {
                view! {
                    <Toggle>
                        {comment.comments.into_iter()
                            .map(|comment: api::Comment| view! { <Comment comment /> })
                            .collect_view()}
                    </Toggle>
                }
            })}
        </li>
    }
}

#[island]
pub fn Toggle(children: Children) -> impl IntoView {
    let (open, set_open) = signal(true);
    view! {
        <div class="toggle" class:open=open>
            <a on:click=move |_| set_open.update(|n| *n = !*n)>
                {move || if open.get() {
                    "[-]"
                } else {
                    "[+] comments collapsed"
                }}
            </a>
        </div>
        <ul
            class="comment-children"
            style:display=move || if open.get() {
                "block"
            } else {
                "none"
            }
        >
            {children()}
        </ul>
    }
}
