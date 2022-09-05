use crate::api;
use leptos::*;

pub fn user_data(
    cx: Scope,
    params: Memo<ParamsMap>,
    _location: Location,
) -> Resource<String, Result<api::User, ()>> {
    log::debug!("(story_data) loading data for user");
    create_resource(
        cx,
        move || params().get("id").cloned().unwrap_or_default(),
        |id| async move { api::fetch_api(&api::user(&id)).await },
    )
}

#[component]
pub fn User(cx: Scope) -> Element {
    let user = use_loader::<Resource<String, Result<api::User, ()>>>(cx);
    view! {
        <div class="user-view">
            {move || user.read().map(|user| match user {
                Err(_) => view! { <h1>"User not found."</h1> },
                Ok(user) => view! {
                    <div>
                        <h1>"User: " {user.id}</h1>
                        <ul class="meta">
                            <li>
                                <span class="label">"Created: "</span> {user.created}
                            </li>
                            <li>
                            <span class="label">"Karma: "</span> {user.karma}
                            </li>
                            //{user.about.map(|about| view! { <li inner_html={user.about} class="about"></li> })}
                        </ul>
                       /*  <p class="links">
                            <a href={format!("https://news.ycombinator.com/submitted?id={}", user.id)}>"submissions"</a>
                            " | "
                            <a href={format!("https://news.ycombinator.com/threads?id={}", user.id)}>"comments"</a>
                        </p> */
                    </div>
                }
            })}
        </div>
    }
}
