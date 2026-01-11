use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    StaticSegment,
    components::{FlatRoutes, Route, Router},
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || {
        view! {
            <Title text="Error from fallback"/>
            <h1>"This is fallback rendering."</h1>
        }
        .into_view()
    };

    view! {
        <Router>
            <nav>
                <a href="/">"Home"</a>
            </nav>
            <main>
                <FlatRoutes fallback>
                    <Route path=StaticSegment("") view=HomePage/>
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Home Page"</h1>
    }
}
