use leptos::prelude::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options islands=true/>
                <link rel="stylesheet" id="leptos" href="/pkg/islands.css"/>
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico"/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <script src="/routing.js"></script>
        <Router>
            <header>
                <h1>"My Application"</h1>
            </header>
            <nav>
                <a href="/">"Page A"</a>
                <a href="/b">"Page B"</a>
            </nav>
            <main>
                <p>
                    <label>"Home Checkbox" <input type="checkbox"/></label>
                </p>
                <FlatRoutes fallback=|| "Not found.">
                    <Route path=StaticSegment("") view=PageA/>
                    <Route path=StaticSegment("b") view=PageB/>
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
pub fn PageA() -> impl IntoView {
    view! { <label>"Page A" <input type="checkbox"/></label> }
}

#[component]
pub fn PageB() -> impl IntoView {
    view! { <label>"Page B" <input type="checkbox"/></label> }
}
