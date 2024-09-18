use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{ProtectedParentRoute, Route, Router, Routes, A},
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

    let x = untrack(|| "foo");

    view! {
        <Router>
            <nav>
                <A href="/">"Home"</A>
                " | "
                <A href="/dashboard">"Dashboard"</A>
                " | "
                <A href="/profile">"Profile"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <ProtectedParentRoute
                        path=StaticSegment("")
                        view=|| view! { <div>"Protected Content"</div> }
                        condition=move || Some(false)
                        redirect_path=|| "/".to_string()
                    >
                        <Route path=StaticSegment("dashboard") view=DashboardPage/>
                        <Route path=StaticSegment("profile") view=ProfilePage/>
                    </ProtectedParentRoute>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Welcome to the Home Page"</h1>
        <p>"This page is accessible to everyone."</p>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <h1>"Dashboard"</h1>
        <p>"This is a protected page. You should only see this if you're authenticated."</p>
    }
}

#[component]
fn ProfilePage() -> impl IntoView {
    view! {
        <h1>"Profile"</h1>
        <p>"This is another protected page. You should only see this if you're authenticated."</p>
    }
}
