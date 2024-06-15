use crate::Page;
use api_boundary::UserInfo;
use leptos::prelude::*;
use leptos_router::*;

#[component]
pub fn Home(user_info: Signal<Option<UserInfo>>) -> impl IntoView {
    view! {
        <h2>"Leptos Login example"</h2>
        {move || match user_info.get() {
            Some(info) => {
                view! { <p>"You are logged in with " {info.email} "."</p> }
                    .into_view()
            }
            None => {
                view! {
                    <p>"You are not logged in."</p>
                    <A href=Page::Login.path()>"Login now."</A>
                }
                    .into_view()
            }
        }}
    }
}
