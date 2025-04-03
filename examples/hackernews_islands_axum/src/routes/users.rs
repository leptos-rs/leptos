use crate::api;
use leptos::{either::Either, prelude::*, server::Resource};
use leptos_router::hooks::use_params_map;

#[server]
pub async fn fetch_user(
    id: String,
) -> Result<Option<api::User>, ServerFnError> {
    Ok(api::fetch_api::<api::User>(&api::user(&id)).await)
}

#[component]
pub fn User() -> impl IntoView {
    let params = use_params_map();
    let user = Resource::new(
        move || params.read().get("id").unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                Ok(None)
            } else {
                fetch_user(id).await
            }
        },
    );
    view! {
        <div class="user-view">
            <Suspense fallback=|| view! { "Loading..." }>
                {move || Suspend::new(async move { match user.await.ok().flatten() {
                    None => Either::Left(view! {  <h1>"User not found."</h1> }),
                    Some(user) => Either::Right(view! {
                        <div>
                            <h1>"User: " {user.id.clone()}</h1>
                            <ul class="meta">
                                <li>
                                    <span class="label">"Created: "</span> {user.created}
                                </li>
                                <li>
                                <span class="label">"Karma: "</span> {user.karma}
                                </li>
                                <li inner_html={user.about} class="about"></li>
                            </ul>
                            <p class="links">
                                <a href=format!("https://news.ycombinator.com/submitted?id={}", user.id)>"submissions"</a>
                                " | "
                                <a href=format!("https://news.ycombinator.com/threads?id={}", user.id)>"comments"</a>
                            </p>
                        </div>
                    })
                }})}
            </Suspense>
        </div>
    }
}
