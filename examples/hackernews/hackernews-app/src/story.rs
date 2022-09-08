use crate::api;
use leptos::*;

pub fn story_data(
    cx: Scope,
    params: Memo<ParamsMap>,
    _location: Location,
) -> Resource<String, Result<api::Story, ()>> {
    log::debug!("(story_data) loading data for story");
    create_resource(
        cx,
        move || params().get("id").cloned().unwrap_or_default(),
        |id| async move { api::fetch_api(&api::story(&format!("item/{id}"))).await },
    )
}

#[component]
pub fn Story(cx: Scope) -> Element {
    let story = use_loader::<Resource<String, Result<api::Story, ()>>>(cx);

    view! {
        <div>
            {move || story.read().map(|story| match story {
                Err(_) => view! { <div class="item-view">"Error loading this story."</div> },
                Ok(story) => view! {
                    <div class="item-view">
                        <div class="item-view-header">
                        <a href={story.url} target="_blank">
                            <h1>{story.title}</h1>
                        </a>
                        <span class="host">
                            "("{story.domain}")"
                        </span>
                        {story.user.map(|user| view! { <p class="meta">
                            // TODO issue here in renderer
                            {story.points}
                            " points | by "
                            <Link to=format!("/users/{}", user)>{&user}</Link>
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
                            <For each={move || story.comments.clone().unwrap_or_default()} key={|comment| comment.id}>
                                {move |cx, comment: &api::Comment| view! { <Comment comment={comment.clone()} /> }}
                            </For>
                        </ul>
                    </div>
                </div>
            }})}
        </div>
    }
}

#[component]
pub fn Comment(cx: Scope, comment: api::Comment) -> Element {
    let (open, set_open) = create_signal(cx, true);

    view! {
        <li class="comment">
        <div class="by">
            <Link to={format!("/users/{}", comment.user)}>{&comment.user}</Link>
            {format!(" {}", comment.time_ago)}
        </div>
        <div class="text" inner_html={comment.content}></div>
        {(!comment.comments.is_empty()).then(|| {
            view! {
                <div>
                    <div class="toggle" class:open=open>
                        <a on:click=move |_| set_open(|n| *n = !*n)>
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
                        move || view! {
                            <ul class="comment-children">
                                <For each={move || comments.clone()} key=|comment| comment.id>
                                    {|cx, comment: &api::Comment| view! {
                                        <Comment comment={comment.clone()} />
                                    }}
                                </For>
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
