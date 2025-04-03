use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{FlatRoutes, ProtectedRoute, Route, Router},
    hooks::use_params,
    params::Params,
    ParamSegment, SsrMode, StaticSegment,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use thiserror::Error;

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

#[cfg(feature = "ssr")]
static IS_ADMIN: AtomicBool = AtomicBool::new(true);

#[server]
pub async fn is_admin() -> Result<bool, ServerFnError> {
    Ok(IS_ADMIN.load(Ordering::Relaxed))
}

#[server]
pub async fn set_is_admin(is_admin: bool) -> Result<(), ServerFnError> {
    IS_ADMIN.store(is_admin, Ordering::Relaxed);
    Ok(())
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();
    let toggle_admin = ServerAction::<SetIsAdmin>::new();
    let is_admin =
        Resource::new(move || toggle_admin.version().get(), |_| is_admin());

    view! {
        <Stylesheet id="leptos" href="/pkg/ssr_modes.css"/>
        <Title text="Welcome to Leptos"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <nav>
                <a href="/">"Home"</a>
                <a href="/admin">"Admin"</a>
                <Transition>
                    <ActionForm action=toggle_admin>
                        <input
                            type="hidden"
                            name="is_admin"
                            value=move || {
                                (!is_admin.get().and_then(|n| n.ok()).unwrap_or_default())
                                    .to_string()
                            }
                        />

                        <button>
                            {move || {
                                if is_admin.get().and_then(Result::ok).unwrap_or_default() {
                                    "Log Out"
                                } else {
                                    "Log In"
                                }
                            }}

                        </button>
                    </ActionForm>
                </Transition>
            </nav>
            <main>
                <FlatRoutes fallback>
                    // We’ll load the home page with out-of-order streaming and <Suspense/>
                    <Route path=StaticSegment("") view=HomePage/>

                    // We'll load the posts with async rendering, so they can set
                    // the title and metadata *after* loading the data
                    <Route
                        path=(StaticSegment("post"), ParamSegment("id"))
                        view=Post
                        ssr=SsrMode::Async
                    />
                    <Route
                        path=(StaticSegment("post_in_order"), ParamSegment("id"))
                        view=Post
                        ssr=SsrMode::InOrder
                    />
                    <Route
                        path=(StaticSegment("post_partially_blocked"), ParamSegment("id"))
                        view=Post
                    />
                    <ProtectedRoute
                        path=StaticSegment("admin")
                        view=Admin
                        ssr=SsrMode::Async
                        condition=move || is_admin.get().map(|n| n.unwrap_or(false))
                        redirect_path=|| "/"
                    />
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    // load the posts
    let posts = Resource::new(|| (), |_| list_post_metadata());
    let posts = move || {
        posts
            .get()
            .map(|n| n.unwrap_or_default())
            .unwrap_or_default()
    };

    let posts2 = Resource::new(|| (), |_| list_post_metadata());
    let posts2 = Resource::new(
        || (),
        move |_| async move { posts2.await.as_ref().map(Vec::len).unwrap_or(0) },
    );

    view! {
        <h1>"My Great Blog"</h1>
        <Suspense fallback=move || view! { <p>"Loading posts..."</p> }>
            <p>"number of posts: " {Suspend::new(async move { posts2.await })}</p>
        </Suspense>
        <Suspense fallback=move || view! { <p>"Loading posts..."</p> }>
            <ul>
                <For each=posts key=|post| post.id let:post>
                    <li>
                        <a href=format!("/post/{}", post.id)>{post.title.clone()}</a>
                        "|"
                        <a href=format!(
                            "/post_in_order/{}",
                            post.id,
                        )>{post.title.clone()} "(in order)"</a>
                        "|"
                        <a href=format!(
                            "/post_partially_blocked/{}",
                            post.id,
                        )>{post.title} "(partially blocked)"</a>
                    </li>
                </For>
            </ul>
        </Suspense>
    }
}

#[derive(Params, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PostParams {
    id: Option<usize>,
}

#[component]
fn Post() -> impl IntoView {
    let query = use_params::<PostParams>();
    let id = move || {
        query.with(|q| {
            q.as_ref()
                .map(|q| q.id.unwrap_or_default())
                .map_err(|_| PostError::InvalidId)
        })
    };
    let post_resource = Resource::new_blocking(id, |id| async move {
        match id {
            Err(e) => Err(e),
            Ok(id) => get_post(id)
                .await
                .map(|data| data.ok_or(PostError::PostNotFound))
                .map_err(|_| PostError::ServerError),
        }
    });
    let comments_resource = Resource::new(id, |id| async move {
        match id {
            Err(e) => Err(e),
            Ok(id) => {
                get_comments(id).await.map_err(|_| PostError::ServerError)
            }
        }
    });

    let post_view = Suspend::new(async move {
        match post_resource.await {
            Ok(Ok(post)) => {
                Ok(view! {
                    <h1>{post.title.clone()}</h1>
                    <p>{post.content.clone()}</p>

                    // since we're using async rendering for this page,
                    // this metadata should be included in the actual HTML <head>
                    // when it's first served
                    <Title text=post.title/>
                    <Meta name="description" content=post.content/>
                })
            }
            _ => Err(PostError::ServerError),
        }
    });
    let comments_view = Suspend::new(async move {
        match comments_resource.await {
            Ok(comments) => Ok(view! {
                <h1>"Comments"</h1>
                <ul>
                    {comments
                        .into_iter()
                        .map(|comment| view! { <li>{comment}</li> })
                        .collect_view()}

                </ul>
            }),
            _ => Err(PostError::ServerError),
        }
    });

    view! {
        <em>"The world's best content."</em>
        <Suspense fallback=move || view! { <p>"Loading post..."</p> }>
            <ErrorBoundary fallback=|errors| {
                view! {
                    <div class="error">
                        <h1>"Something went wrong."</h1>
                        <ul>
                            {move || {
                                errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, error)| view! { <li>{error.to_string()}</li> })
                                    .collect::<Vec<_>>()
                            }}

                        </ul>
                    </div>
                }
            }>{post_view}</ErrorBoundary>
        </Suspense>
        <Suspense fallback=move || view! { <p>"Loading comments..."</p> }>{comments_view}</Suspense>
    }
}

#[component]
pub fn Admin() -> impl IntoView {
    view! { <p>"You can only see this page if you're logged in."</p> }
}

// Dummy API

static POSTS: LazyLock<[Post; 3]> = LazyLock::new(|| {
    [
        Post {
            id: 0,
            title: "My first post".to_string(),
            content: "This is my first post".to_string(),
        },
        Post {
            id: 1,
            title: "My second post".to_string(),
            content: "This is my second post".to_string(),
        },
        Post {
            id: 2,
            title: "My third post".to_string(),
            content: "This is my third post".to_string(),
        },
    ]
});

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostError {
    #[error("Invalid post ID.")]
    InvalidId,
    #[error("Post not found.")]
    PostNotFound,
    #[error("Server error.")]
    ServerError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    id: usize,
    title: String,
    content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostMetadata {
    id: usize,
    title: String,
}

#[server]
pub async fn list_post_metadata() -> Result<Vec<PostMetadata>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(POSTS
        .iter()
        .map(|data| PostMetadata {
            id: data.id,
            title: data.title.clone(),
        })
        .collect())
}

#[server]
pub async fn get_post(id: usize) -> Result<Option<Post>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(POSTS.iter().find(|post| post.id == id).cloned())
}

#[server]
pub async fn get_comments(id: usize) -> Result<Vec<String>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    _ = id;
    Ok(vec!["Some comment".into(), "Some other comment".into()])
}
