// This is essentially a port of Solid's Hacker News demo
// https://github.com/solidjs/solid-hackernews

use leptos::*;

mod api;
mod nav;
mod stories;
mod story;
mod users;
use nav::*;
use stories::*;
use story::*;
use users::*;

#[component]
pub fn App(cx: Scope) -> Vec<Branch> {
    view! { cx,
        <Routes>
            <Route path="" element=|cx| view! { cx,  <Main/> }>
                <Route path="users/:id" element=|cx| view! { cx,  <User/> } loader=user_data.into() />
                <Route path="stories/:id" element=|cx| view! { cx,  <Story/> } loader=story_data.into() />
                <Route path="*stories" element=|cx| view! { cx,  <Stories/> } loader=stories_data.into()/>
            </Route>
        </Routes>
    }
}

#[component]
pub fn Main(cx: Scope) -> Element {
    view! { cx,
        <article>
            <Nav />
            <Outlet />
        </article>
    }
}
