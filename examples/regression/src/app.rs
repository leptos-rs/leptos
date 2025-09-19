use crate::{
    issue_4005::Routes4005, issue_4088::Routes4088, issue_4217::Routes4217,
    issue_4285::Routes4285, issue_4296::Routes4296, pr_4015::Routes4015,
    pr_4091::Routes4091,
};
use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
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
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();
    let (_, set_is_routing) = signal(false);

    view! {
        <Stylesheet id="leptos" href="/pkg/regression.css"/>
        <Router set_is_routing>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=HomePage/>
                    <Routes4091/>
                    <Routes4015/>
                    <Routes4088/>
                    <Routes4217/>
                    <Routes4005/>
                    <Routes4285/>
                    <Routes4296/>
                </Routes>
            </main>
        </Router>
    }
}

#[server]
async fn server_call() -> Result<(), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    Ok(())
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <Title text="Regression Tests"/>
        <h1>"Listing of regression tests"</h1>
        <nav>
            <ul>
                <li><a href="/4091/">"4091"</a></li>
                <li><a href="/4015/">"4015"</a></li>
                <li><a href="/4088/">"4088"</a></li>
                <li><a href="/4217/">"4217"</a></li>
                <li><a href="/4005/">"4005"</a></li>
                <li><a href="/4285/">"4285"</a></li>
                <li><a href="/4296/">"4296"</a></li>
            </ul>
        </nav>
    }
}
