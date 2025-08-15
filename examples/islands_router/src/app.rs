use leptos::{
    either::{Either, EitherOf3},
    prelude::*,
};
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::{use_params_map, use_query_map},
    path,
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
                <HydrationScripts options=options islands=true islands_router=true/>
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
                    <Route path=path!("") view=Home/>
                    <Route path=path!("user/:id") view=Details/>
                    <Route path=path!("about") view=About/>
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

#[server]
pub async fn delete_user(id: u32) -> Result<(), ServerFnError> {
    let users = tokio::fs::read_to_string("./mock_data.json").await?;
    let mut data: Vec<User> = serde_json::from_str(&users)?;
    data.retain(|user| user.id != id);
    let new_json = serde_json::to_string(&data)?;
    tokio::fs::write("./mock_data.json", &new_json).await?;
    Ok(())
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
    let delete_user_action = ServerAction::<DeleteUser>::new();

    let view = move || {
        Suspend::new(async move {
            let users = data.await.unwrap();
            if q().is_none() {
                EitherOf3::A(view! {
                    <p class="note">"Enter a search to begin viewing contacts."</p>
                })
            } else if users.is_empty() {
                EitherOf3::B(view! {
                    <p class="note">"No users found matching that search."</p>
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
                                    <td>{user.first_name}</td>
                                    <td>{user.last_name}</td>
                                    <td>{user.email}</td>
                                    <td>
                                        <a href=format!("/user/{}", user.id)>"Details"</a>
                                        <input type="checkbox"/>
                                        <ActionForm action=delete_user_action>
                                            <input type="hidden" name="id" value=user.id/>
                                            <input type="submit" value="Delete"/>
                                        </ActionForm>
                                    </td>
                                </tr>
                            </For>
                        </tbody>
                    </table>
                })
            }
        })
    };
    view! {
        <section class="page">
            <form method="GET" class="search">
                <input type="search" name="q" value=q autofocus oninput="this.form.requestSubmit()"/>
                <input type="submit"/>
            </form>
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>{view}</Suspense>
        </section>
    }
}

#[component]
pub fn Details() -> impl IntoView {
    #[server]
    pub async fn get_user(id: u32) -> Result<Option<User>, ServerFnError> {
        let users = tokio::fs::read_to_string("./mock_data.json").await?;
        let data: Vec<User> = serde_json::from_str(&users)?;
        Ok(data.iter().find(|user| user.id == id).cloned())
    }
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| id.parse::<u32>().ok())
    };
    let user = Resource::new(id, |id| async move {
        match id {
            None => Ok(None),
            Some(id) => get_user(id).await,
        }
    });

    move || {
        Suspend::new(async move {
            user.await.map(|user| match user {
                None => Either::Left(view! {
                    <section class="page">
                        <h2>"Not found."</h2>
                        <p>"Sorry — we couldn’t find that user."</p>
                    </section>
                }),
                Some(user) => Either::Right(view! {
                    <section class="page">
                        <h2>{user.first_name} " " { user.last_name}</h2>
                        <p class="email">{user.email}</p>
                    </section>
                }),
            })
        })
    }
}

#[component]
pub fn About() -> impl IntoView {
    view! {
        <section class="page">
            <h2>"About"</h2>
            <p>"This demo is intended to show off an experimental “islands router” feature, which mimics the smooth transitions and user experience of client-side routing while minimizing the amount of code that actually runs in the browser."</p>
            <p>"By default, all the content in this application is only rendered on the server. But you can add client-side interactivity via islands like this one:"</p>
            <Counter/>
        </section>
    }
}

#[island]
pub fn Counter() -> impl IntoView {
    let count = RwSignal::new(0);
    view! {
        <button class="counter" on:click=move |_| *count.write() += 1>{count}</button>
    }
}
