use crate::Page;
use api_boundary::UserInfo;
use leptos::*;
use leptos_router::*;

#[component]
pub fn Home(cx: Scope, user_info: Signal<Option<UserInfo>>) -> impl IntoView {
    view! { cx,
        <h2>"Leptos Login example"</h2>
        {move || match user_info.get() {
            Some(info) => {
                view! { cx, <p>"You are logged in with " {info.email} "."</p> }
                    .into_view(cx)
            }
            None => {
                view! { cx,
                    <p>"You are not logged in."</p>
                    <A href=Page::Login.path()>"Login now."</A>
                }
                    .into_view(cx)
            }
        }}
    }
}
