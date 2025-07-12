use leptos::{prelude::*, task::spawn_local};
use leptos_router::{
    components::{FlatRoutes, Route, Router, Routes},
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
            <a href="/c">"C"</a>
            <span style="float: right" id="navigating">
                {move || is_routing.get().then_some("Navigating...")}
            </span>
        </nav>
        <Router set_is_routing>
            <Routes fallback=|| "Not found.">
                <Route path=StaticSegment("") view=ViewA/>
                <Route path=StaticSegment("b") view=ViewB/>
                <Route path=StaticSegment("c") view={Lazy::<ViewC>::new()}/>
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
    view! { <p id="page">"View A"</p> }
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
async fn deserialize_comments(data: &str) -> Vec<Comment> {
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
        deserialize_comments(&data).await
    });
    view! {
        <p id="page">"View B"</p>
        <Suspense fallback=|| view! { <p id="loading">"Loading..."</p> }>
            <pre>{move || Suspend::new(async move {
                format!("{:#?}", data.await)
            })}</pre>
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
        Self {
            data: LocalResource::new(|| async {
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
            }),
        }
    }

    async fn view(self) -> AnyView {
        let albums = move || {
            Suspend::new(async move {
                this.data
                    .await
                    .into_iter()
                    .map(|album| {
                        view! {
                            <li>{album.title}</li>
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
