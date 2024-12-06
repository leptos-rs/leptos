use leptos::{either::EitherOf3, prelude::*};
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::use_query_map,
    SsrMode, StaticSegment,
};
use serde::{Deserialize, Serialize};

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
                <h1>"My Contacts"</h1>
            </header>
            <nav>
                <a href="/">"Home"</a>
                <a href="/about">"About"</a>
            </nav>
            <main>
                <Routes fallback=|| "Not found.">
                    <Route path=StaticSegment("") view=Home ssr=SsrMode::Async/>
                    <Route path=StaticSegment("about") view=About ssr=SsrMode::Async/>
                </Routes>
            </main>
        </Router>
    }
}

#[server]
pub async fn search(query: String) -> Result<Vec<User>, ServerFnError> {
    let users = tokio::fs::read_to_string("./mock_data.json").await?;
    let data: Vec<User> = serde_json::from_str(&users)?;
    let query = query.to_ascii_lowercase();
    Ok(data
        .into_iter()
        .filter(|user| {
            user.first_name.to_ascii_lowercase().contains(&query)
                || user.last_name.to_ascii_lowercase().contains(&query)
                || user.email.to_ascii_lowercase().contains(&query)
        })
        .collect())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    id: u32,
    first_name: String,
    last_name: String,
    email: String,
}

#[component]
pub fn Home() -> impl IntoView {
    let q = use_query_map();
    let q = move || q.read().get("q");
    let data = Resource::new(q, |q| async move {
        if let Some(q) = q {
            search(q).await
        } else {
            Ok(vec![])
        }
    });
    let view = move || {
        Suspend::new(async move {
            let users = data.await.unwrap();
            if q().is_none() {
                EitherOf3::A(view! {
                    <p>"Enter a search to begin viewing contacts."</p>
                })
            } else if users.is_empty() {
                EitherOf3::B(view! {
                    <p>"No users found matching that search."</p>
                })
            } else {
                EitherOf3::C(view! {
                    <table>
                        <tbody>
                            <For
                                each=move || users.clone()
                                key=|user| user.id
                                let:user
                            >
                                <tr>
                                    <td><input type="checkbox"/></td>
                                    <td>{user.first_name}</td>
                                    <td>{user.last_name}</td>
                                    <td>{user.email}</td>
                                </tr>
                            </For>
                        </tbody>
                    </table>
                })
            }
        })
    };
    view! {
        <form method="GET">
            <input type="search" name="q" value=q autofocus oninput="this.form.requestSubmit()"/>
            <input type="submit"/>
        </form>
        <Suspense fallback=|| "Loading...">{view}</Suspense>
    }
}

#[component]
pub fn About() -> impl IntoView {
    view! {
        <h1>"About"</h1>
        <p>"This demo is intended to show off an experimental “islands router” feature, which mimics the smooth transitions and user experience of client-side routing while minimizing the amount of code that actually runs in the browser."</p>
    }
}
