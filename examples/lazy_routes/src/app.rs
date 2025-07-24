use leptos::{prelude::*, task::spawn_local};
use leptos_router::{
    components::{Outlet, ParentRoute, Route, Router, Routes},
    lazy_route, Lazy, LazyRoute, StaticSegment,
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
                <HydrationScripts options/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let count = RwSignal::new(0);
    provide_context(count);
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <nav id="nav" style="width: 100%">
            <a href="/">"A"</a> " | "
            <a href="/b">"B"</a> " | "
            <a href="/c">"C"</a> " | "
            <a href="/d">"D"</a>
            <span style="float: right" id="navigating">
                {move || is_routing.get().then_some("Navigating...")}
            </span>
        </nav>
        <Router set_is_routing>
            <Routes fallback=|| "Not found.">
                <Route path=StaticSegment("") view=ViewA/>
                <Route path=StaticSegment("b") view=ViewB/>
                <Route path=StaticSegment("c") view={Lazy::<ViewC>::new()}/>
                // you can nest lazy routes, and there data and views will all load concurrently
                <ParentRoute path=StaticSegment("d") view={Lazy::<ViewD>::new()}>
                    <Route path=StaticSegment("") view={Lazy::<ViewE>::new()}/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

// View A: A plain old synchronous route, just like they all currently work. The WASM binary code
// for this is shipped as part of the main bundle.  Any data-loading code (like resources that run
// in the body of the component) will be shipped as part of the main bundle.

#[component]
pub fn ViewA() -> impl IntoView {
    leptos::logging::log!("View A");
    let result = RwSignal::new("Click a button to see the result".to_string());

    view! {
        <p id="page">"View A"</p>
        <pre id="result">{result}</pre>
        <button id="First" on:click=move |_| spawn_local(async move { result.set(first_value().await); })>"First"</button>
        <button id="Second" on:click=move |_| spawn_local(async move { result.set(second_value().await); })>"Second"</button>
        // test to make sure duplicate names in different scopes can be used
        <button id="Third" on:click=move |_| {
            #[lazy]
            pub fn second_value() -> String {
                "Third value.".to_string()
            }

            spawn_local(async move {
                result.set(second_value().await);
            });
        }>"Third"</button>
    }
}

// View B: lazy-loaded route with lazy-loaded data
#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    #[serde(rename = "postId")]
    post_id: usize,
    id: usize,
    name: String,
    email: String,
    body: String,
}

#[lazy]
fn deserialize_comments(data: &str) -> Vec<Comment> {
    serde_json::from_str(data).unwrap()
}

#[component]
pub fn ViewB() -> impl IntoView {
    let data = LocalResource::new(|| async move {
        let preload = deserialize_comments("[]");
        let (_, data) = futures::future::join(preload, async {
            gloo_timers::future::TimeoutFuture::new(500).await;

            r#"
                [
                    {
                        "postId": 1,
                        "id": 1,
                        "name": "id labore ex et quam laborum",
                        "email": "Eliseo@gardner.biz",
                        "body": "laudantium enim quasi est quidem magnam voluptate ipsam eos\ntempora quo necessitatibus\ndolor quam autem quasi\nreiciendis et nam sapiente accusantium"
                    },
                    {
                        "postId": 1,
                        "id": 2,
                        "name": "quo vero reiciendis velit similique earum",
                        "email": "Jayne_Kuhic@sydney.com",
                        "body": "est natus enim nihil est dolore omnis voluptatem numquam\net omnis occaecati quod ullam at\nvoluptatem error expedita pariatur\nnihil sint nostrum voluptatem reiciendis et"
                    },
                    {
                        "postId": 1,
                        "id": 3,
                        "name": "odio adipisci rerum aut animi",
                        "email": "Nikita@garfield.biz",
                        "body": "quia molestiae reprehenderit quasi aspernatur\naut expedita occaecati aliquam eveniet laudantium\nomnis quibusdam delectus saepe quia accusamus maiores nam est\ncum et ducimus et vero voluptates excepturi deleniti ratione"
                    }
                ]
            "#
        })
        .await;
        deserialize_comments(data).await
    });
    view! {
        <p id="page">"View B"</p>
        <Suspense fallback=|| view! { <p id="loading">"Loading..."</p> }>
            <ul>
            {move || Suspend::new(async move {
                let items = data.await;
                items.into_iter()
                    .map(|comment| view! {
                        <li id=format!("{}-{}", comment.post_id, comment.id)>
                            <strong>{comment.name}</strong>  " (by " {comment.email} ")"<br/>
                            {comment.body}
                        </li>
                    })
                    .collect_view()
            })}
            </ul>
        </Suspense>
    }
    .into_any()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Album {
    #[serde(rename = "userId")]
    user_id: usize,
    id: usize,
    title: String,
}

// View C: a lazy view, and some data, loaded in parallel when we navigate to /c.
#[derive(Clone)]
pub struct ViewC {
    data: LocalResource<Vec<Album>>,
}

// Lazy-loaded routes need to implement the LazyRoute trait. They define a "route data" struct,
// which is created with `::data()`, and then a separate view function which is lazily loaded.
//
// This is important because it allows us to concurrently 1) load the route data, and 2) lazily
// load the component, rather than creating a "waterfall" where we can't start loading the route
// data until we've received the view.
//
// The `#[lazy_route]` macro makes `view` into a lazy-loaded inner function, replacing `self` with
// `this`.
#[lazy_route]
impl LazyRoute for ViewC {
    fn data() -> Self {
        // the data method itself is synchronous: it typically creates things like Resources,
        // which are created synchronously but spawn an async data-loading task
        // if you want further code-splitting, however, you can create a lazy function to load the data!
        #[lazy]
        async fn lazy_data() -> Vec<Album> {
            gloo_timers::future::TimeoutFuture::new(250).await;
            vec![
                Album {
                    user_id: 1,
                    id: 1,
                    title: "quidem molestiae enim".into(),
                },
                Album {
                    user_id: 1,
                    id: 2,
                    title: "sunt qui excepturi placeat culpa".into(),
                },
                Album {
                    user_id: 1,
                    id: 3,
                    title: "omnis laborum odio".into(),
                },
            ]
        }

        Self {
            data: LocalResource::new(lazy_data),
        }
    }

    fn view(this: Self) -> AnyView {
        let albums = move || {
            Suspend::new(async move {
                this.data
                    .await
                    .into_iter()
                    .map(|album| {
                        view! {
                            <li id=format!("{}-{}", album.user_id, album.id)>
                                {album.title}
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
            })
        };
        view! {
            <p id="page">"View C"</p>
            <hr/>
            <Suspense fallback=|| view! { <p id="loading">"Loading..."</p> }>
                <ul>{albums}</ul>
            </Suspense>
        }
        .into_any()
    }
}

// When two functions have shared code, that shared code will be split out automatically
// into an additional file. For example, the shared serde code here will be split into a single file,
// and then loaded lazily once when the first of the two functions is called

#[lazy]
pub fn first_value() -> String {
    #[derive(Serialize)]
    struct FirstValue {
        a: String,
        b: i32,
    }

    serde_json::to_string(&FirstValue {
        a: "First Value".into(),
        b: 1,
    })
    .unwrap()
}

#[lazy]
pub fn second_value() -> String {
    #[derive(Serialize)]
    struct SecondValue {
        a: String,
        b: i32,
    }

    serde_json::to_string(&SecondValue {
        a: "Second Value".into(),
        b: 2,
    })
    .unwrap()
}

struct ViewD {
    data: Resource<Result<Vec<i32>, ServerFnError>>,
}

#[lazy_route]
impl LazyRoute for ViewD {
    fn data() -> Self {
        Self {
            data: Resource::new(|| (), |_| d_data()),
        }
    }

    fn view(this: Self) -> AnyView {
        let items = move || {
            Suspend::new(async move {
                this.data
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| view! { <li>{item}</li> })
                    .collect::<Vec<_>>()
            })
        };
        view! {
            <p id="page">"View D"</p>
            <hr/>
            <Suspense fallback=|| view! { <p id="loading">"Loading..."</p> }>
                <ul>{items}</ul>
            </Suspense>
            <Outlet/>
        }
        .into_any()
    }
}

// Server functions can be made lazy by combining the two macros,
// with `#[server]` coming first, then `#[lazy]`
#[server]
#[lazy]
async fn d_data() -> Result<Vec<i32>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(vec![1, 1, 2, 3, 5, 8, 13])
}

struct ViewE {
    data: Resource<Result<Vec<String>, ServerFnError>>,
}

#[lazy_route]
impl LazyRoute for ViewE {
    fn data() -> Self {
        Self {
            data: Resource::new(|| (), |_| e_data()),
        }
    }

    fn view(this: Self) -> AnyView {
        let items = move || {
            Suspend::new(async move {
                this.data
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| view! { <li>{item}</li> })
                    .collect::<Vec<_>>()
            })
        };
        view! {
            <p id="page">"View E"</p>
            <hr/>
            <Suspense fallback=|| view! { <p id="loading">"Loading..."</p> }>
                <ul>{items}</ul>
            </Suspense>
        }
        .into_any()
    }
}

#[server]
async fn e_data() -> Result<Vec<String>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(vec!["foo".into(), "bar".into(), "baz".into()])
}
