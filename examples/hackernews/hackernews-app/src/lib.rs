// This is essentially a port of a Solid Hacker News demo
// https://github.com/solidjs/solid-hackernews

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

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
pub fn App(cx: Scope) -> Element {
    provide_context(cx, MetaContext::default());

    view! {
        cx,
        <div>
            <Stylesheet href="/static/style.css".into()/>
            <Router>
                <Nav />
                <main>
                    <Routes>
                        <Route path="users/:id" element=|cx| view! { cx,  <User/> } loader=user_data.into() />
                        <Route path="stories/:id" element=|cx| view! { cx,  <Story/> } loader=story_data.into() />
                        <Route path="*stories" element=|cx| view! { cx,  <Stories/> } loader=stories_data.into()/>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
