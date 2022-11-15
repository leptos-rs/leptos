use crate::api::{self, User};
use leptos::*;
use leptos_router::*;

#[component]
pub fn User(cx: Scope) -> Element {
    let params = use_params_map(cx);
    let user = create_resource(
        cx,
        move || params().get("id").cloned().unwrap_or_default(),
        move |id| async move { api::fetch_api::<User>(&api::user(&id)).await },
    );
    view! { cx,
        <div class="user-view">
            {move || user.read().map(|user| match user {
                None => view! { cx,  <h1>"User not found."</h1> },
                Some(user) => view! { cx,
                    <div>
                        <h1>"User: " {&user.id}</h1>
                        <ul class="meta">
                            <li>
                                <span class="label">"Created: "</span> {user.created}
                            </li>
                            <li>
                            <span class="label">"Karma: "</span> {user.karma}
                            </li>
                            {user.about.as_ref().map(|about| view! { cx,  <li inner_html=about class="about"></li> })}
                        </ul>
                        <p class="links">
                            <a href=format!("https://news.ycombinator.com/submitted?id={}", user.id)>"submissions"</a>
                            " | "
                            <a href=format!("https://news.ycombinator.com/threads?id={}", user.id)>"comments"</a>
                        </p>
                    </div>
                }
            })}
        </div>
    }
}
