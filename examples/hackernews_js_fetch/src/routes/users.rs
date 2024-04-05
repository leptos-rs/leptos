use crate::api::{self, User};
use leptos::*;
use leptos_router::*;

#[component]
pub fn User() -> impl IntoView {
    let params = use_params_map();
    let user = create_resource(
        move || params.get().get("id").cloned().unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                None
            } else {
                api::fetch_api::<User>(&api::user(&id)).await
            }
        },
    );
    view! {
        <div class="user-view">
            <Suspense fallback=|| view! {  "Loading..." }>
                {move || user.get().map(|user| match user {
                    None => view! { <h1>"User not found."</h1> }.into_any(),
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
                                {user.about.as_ref().map(|about| view! { <li inner_html=about class="about"></li> })}
                            </ul>
                            <p class="links">
                                <a href=format!("https://news.ycombinator.com/submitted?id={}", user.id)>"submissions"</a>
                                " | "
                                <a href=format!("https://news.ycombinator.com/threads?id={}", user.id)>"comments"</a>
                            </p>
                        </div>
                    }.into_any()
                })}
            </Suspense>
        </div>
    }
}
