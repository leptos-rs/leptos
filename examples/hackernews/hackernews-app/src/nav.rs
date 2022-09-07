use leptos::*;

#[component]
pub fn Nav(cx: Scope) -> Element {
    view! {
        <header class="header">
            <nav class="inner">
                <a href="/"> // <Link to="/".into()>
                    <strong>"HN"</strong>
                </a> // </Link>
                <a href="/new"> // <Link to="/new".into()>
                    <strong>"New"</strong>
                </a> // </Link>
                <a href="/show"> // <Link to="/show".into()>
                    <strong>"Show"</strong>
                </a> // </Link>
                <a href="/ask"> // <Link to="/ask".into()>
                    <strong>"Ask"</strong>
                </a> // </Link>
                <a href="/job"> // <Link to="/job".into()>
                    <strong>"Jobs"</strong>
                </a> // </Link>
                <a class="github" href="http://github.com/gbj/leptos" target="_blank" rel="noreferrer">
                    "Built with Leptos"
                </a>
            </nav>
        </header>
    }
}
