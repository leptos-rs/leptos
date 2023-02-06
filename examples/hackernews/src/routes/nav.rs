use leptos::{component, view, IntoView, Scope};
use leptos_router::*;

#[component]
pub fn Nav(cx: Scope) -> impl IntoView {
    view! { cx,
        <header class="header">
            <nav class="inner">
                <A href="/home">
                    <strong>"HN"</strong>
                </A>
                <A href="/new">
                    <strong>"New"</strong>
                </A>
                <A href="/show">
                    <strong>"Show"</strong>
                </A>
                <A href="/ask">
                    <strong>"Ask"</strong>
                </A>
                <A href="/job">
                    <strong>"Jobs"</strong>
                </A>
                <a class="github" href="http://github.com/gbj/leptos" target="_blank" rel="noreferrer">
                    "Built with Leptos"
                </a>
            </nav>
        </header>
    }
}
