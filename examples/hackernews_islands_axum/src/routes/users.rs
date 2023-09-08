#[allow(unused)] // User is unused in WASM build
use crate::api::{self, User};
use leptos::*;
use leptos_router::*;

#[server(FetchUser, "/api")]
pub async fn fetch_user(
    id: String,
) -> Result<Option<api::User>, ServerFnError> {
    Ok(api::fetch_api::<User>(&api::user(&id)).await)
}

#[component]
pub fn User() -> impl IntoView {
    let params = use_params_map();
    let user = create_resource(
        move || params().get("id").cloned().unwrap_or_default(),
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
            <Suspense fallback=|| ()>
                {move || user.get().map(|user| user.map(|user| match user {
                    None => view! {  <h1>"User not found."</h1> }.into_view(),
                    Some(user) => view! {
                        <div>
                            <h1>"User: " {&user.id}</h1>
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
                    }.into_view()
                }))}
            </Suspense>
        </div>
    }
}
