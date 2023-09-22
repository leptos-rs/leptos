use crate::api;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use std::cell::RefCell;

#[server(FetchStory, "/api")]
pub async fn fetch_story(
    id: String,
) -> Result<RefCell<Option<api::Story>>, ServerFnError> {
    Ok(RefCell::new(
        api::fetch_api::<api::Story>(&api::story(&format!("item/{id}"))).await,
    ))
}

#[component]
pub fn Story() -> impl IntoView {
    let params = use_params_map();
    let story = create_resource(
        move || params().get("id").cloned().unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                Ok(RefCell::new(None))
            } else {
                fetch_story(id).await
            }
        },
    );
    let meta_description = move || {
        story
            .map(|story| {
                story
                    .as_ref()
                    .map(|story| {
                        story.borrow().as_ref().map(|story| story.title.clone())
                    })
                    .ok()
            })
            .flatten()
            .flatten()
            .unwrap_or_else(|| "Loading story...".to_string())
    };

    let story_view = move || {
        story.map(|story| {
        story.as_ref().ok().and_then(|story| {
            let story: Option<api::Story> = story.borrow_mut().take();
            story.map(|story| {
                view! {
                    <div class="item-view">
                        <div class="item-view-header">
                        <a href=story.url target="_blank">
                            <h1>{story.title}</h1>
                        </a>
                        <span class="host">
                            "("{story.domain}")"
                        </span>
                        {story.user.map(|user| view! { <p class="meta">
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
                            {story.comments.unwrap_or_default().into_iter()
                                .map(|comment: api::Comment| view! { <Comment comment /> })
                                .collect_view()}
                        </ul>
                    </div>
                </div>
        }})})})
    };

    view! {
        <Meta name="description" content=meta_description/>
        <Suspense fallback=|| ()>
            {story_view}
        </Suspense>
    }
}

#[island]
pub fn Toggle(children: Children) -> impl IntoView {
    let (open, set_open) = create_signal(true);
    view! {
        <div class="toggle" class:open=open>
            <a on:click=move |_| set_open.update(|n| *n = !*n)>
                {move || if open() {
                    "[-]"
                } else {
                    "[+] comments collapsed"
                }}
            </a>
        </div>
        <ul
            class="comment-children"
            style:display=move || if open() {
                "block"
            } else {
                "none"
            }
        >
            {children()}
        </ul>
    }
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
