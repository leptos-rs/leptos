use crate::islands::{Counter, KeyLog};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};

/// htmx 2.0.11 from its CDN, pinned by hash. The script also works with htmx 4.
const HTMX_SRC: &str =
    "https://cdn.jsdelivr.net/npm/htmx.org@2.0.11/dist/htmx.min.js";
const HTMX_HASH: &str =
    "sha384-2OatzQy1H+Zd/IIrjr1TcuDGqLXeHhbooAyJY1KdQMKnr4LZ22k31GBLdYKHmVjg";

/// htmx keeps no page snapshots, so Back and Forward load the page instead of restoring dead islands.
const HTMX_CONFIG: &str =
    r#"{"historyCacheSize":0,"refreshOnHistoryMiss":true}"#;

#[derive(Clone, Copy)]
pub enum Page {
    Home,
    About,
    NotFound,
}

/// A whole page: the shell, with the page's own content in the `<main>` the links swap.
pub fn page(options: LeptosOptions, page: Page) -> impl IntoView {
    provide_meta_context();
    let (title, content) = match page {
        Page::Home => ("Home", view! { <Home/> }.into_any()),
        Page::About => ("About", view! { <About/> }.into_any()),
        Page::NotFound => ("Not found", view! { <NotFound/> }.into_any()),
    };
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                // Written into the head as plain text; a `<title>` in the view would carry a hydration marker.
                <Title text=title/>
                <MetaTags/>
                <meta name="htmx-config" content=HTMX_CONFIG/>
                <script src=HTMX_SRC integrity=HTMX_HASH crossorigin="anonymous" defer></script>
                <script src="/htmx-islands.js" defer></script>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options islands=true/>
                <link rel="stylesheet" id="leptos" href="/pkg/islands_htmx.css"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
            </head>
            <body>
                <header>
                    <h1>"Islands and htmx"</h1>
                    <nav>
                        <PageLink href="/">"Home"</PageLink>
                        <PageLink href="/about">"About"</PageLink>
                    </nav>
                </header>
                <main id="content">{content}</main>
            </body>
        </html>
    }
}

/// What the About page's button fetches: one island as a piece of HTML, with no shell around it.
pub fn fragment() -> impl IntoView {
    view! { <Counter/> }
}

/// A link to another page: htmx fetches it and swaps its `<main>` in; without htmx, a plain link.
#[component]
fn PageLink(href: &'static str, children: Children) -> impl IntoView {
    view! {
        <a
            href=href
            hx-get=href
            hx-target="#content"
            hx-select="#content"
            hx-swap="outerHTML"
            hx-push-url="true"
        >
            {children()}
        </a>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <h2>"Home"</h2>
        <p>
            "The island below listens to the keyboard. Press a key, then go to About: "
            "the island is swapped out, and a few seconds later it is freed."
        </p>
        <KeyLog/>
    }
}

#[component]
fn About() -> impl IntoView {
    view! {
        <h2>"About"</h2>
        <p>
            "The links swap this page's " <code>"<main>"</code> " with htmx; the rest of the page stays. "
            "The button fetches a fragment of HTML that holds an island."
        </p>
        <button hx-get="/fragment" hx-target="#slot" hx-swap="innerHTML">"Load a counter"</button>
        <div id="slot"></div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    // The file handler renders this page for any path it has no file for, so the status is set here.
    #[cfg(feature = "ssr")]
    expect_context::<leptos_axum::ResponseOptions>()
        .set_status(axum::http::StatusCode::NOT_FOUND);
    view! { <h2>"Not found"</h2> }
}
