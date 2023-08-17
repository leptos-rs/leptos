use crate::api::{self, User};
use leptos::*;
use leptos_router::*;

#[component]
pub fn User(cx: Scope) -> impl IntoView {
    let params = use_params_map(cx);
    let user = create_resource(
        cx,
        move || params().get("id").cloned().unwrap_or_default(),
        move |id| async move {
            if id.is_empty() {
                None
            } else {
                api::fetch_api::<User>(cx, &api::user(&id)).await
            }
        },
    );
    view! { cx,
        <div class="user-view">
            <Suspense fallback=|| view! { cx, "Loading..." }>
                {move || user.read(cx).map(|user| match user {
                    None => view! { cx,  <h1>"User not found."</h1> }.into_any(),
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
                    }.into_any()
                })}
            </Suspense>
        </div>
    }
}
