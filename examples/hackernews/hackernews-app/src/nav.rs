use leptos::*;

#[component]
pub fn Nav(cx: Scope) -> Element {
    view! { cx,
        <header class="header">
            <nav class="inner">
                <Link to="/".into()>
                    <strong>"HN"</strong>
                </Link>
                <Link to="/new".into()>
                    <strong>"New"</strong>
                </Link>
                <Link to="/show".into()>
                    <strong>"Show"</strong>
                </Link>
                <Link to="/ask".into()>
                    <strong>"Ask"</strong>
                </Link>
                <Link to="/job".into()>
                    <strong>"Jobs"</strong>
                </Link>
                <a class="github" href="http://github.com/gbj/leptos" target="_blank" rel="noreferrer">
                    "Built with Leptos"
                </a>
            </nav>
        </header>
    }
}
