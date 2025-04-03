use futures::{channel::mpsc, Stream};
use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{FlatRoutes, Redirect, Route, Router},
    hooks::use_params,
    params::Params,
    path,
    static_routes::StaticRoute,
    SsrMode,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/ssr_modes.css"/>
        <Title text="Welcome to Leptos"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <nav>
                <a href="/">"Home"</a>
            </nav>
            <main>
                <FlatRoutes fallback>
                    <Route
                        path=path!("/")
                        view=HomePage
                        ssr=SsrMode::Static(
                            StaticRoute::new().regenerate(|_| watch_path(Path::new("./posts"))),
                        )
                    />

                    <Route
                        path=path!("/about")
                        view=move || view! { <Redirect path="/"/> }
                        ssr=SsrMode::Static(StaticRoute::new())
                    />

                    <Route
                        path=path!("/post/:slug/")
                        view=Post
                        ssr=SsrMode::Static(
                            StaticRoute::new()
                                .prerender_params(|| async move {
                                    [("slug".into(), list_slugs().await.unwrap_or_default())]
                                        .into_iter()
                                        .collect()
                                })
                                .regenerate(|params| {
                                    let slug = params.get("slug").unwrap();
                                    watch_path(Path::new(&format!("./posts/{slug}.md")))
                                }),
                        )
                    />

                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    // load the posts
    let posts = Resource::new(|| (), |_| list_posts());
    let posts = move || {
        posts
            .get()
            .map(|n| n.unwrap_or_default())
            .unwrap_or_default()
    };

    view! {
        <h1>"My Great Blog"</h1>
        <Suspense fallback=move || view! { <p>"Loading posts..."</p> }>
            <ul>
                <For each=posts key=|post| post.slug.clone() let:post>
                    <li>
                        <a href=format!("/post/{}/", post.slug)>{post.title.clone()}</a>
                    </li>
                </For>
            </ul>
        </Suspense>
    }
}

#[derive(Params, Clone, Debug, PartialEq, Eq)]
pub struct PostParams {
    slug: Option<String>,
}

#[component]
fn Post() -> impl IntoView {
    let query = use_params::<PostParams>();
    let slug = move || {
        query
            .get()
            .map(|q| q.slug.unwrap_or_default())
            .map_err(|_| PostError::InvalidId)
    };
    let post_resource = Resource::new_blocking(slug, |slug| async move {
        match slug {
            Err(e) => Err(e),
            Ok(slug) => get_post(slug)
                .await
                .map(|data| data.ok_or(PostError::PostNotFound))
                .map_err(|e| PostError::ServerError(e.to_string())),
        }
    });

    let post_view = move || {
        Suspend::new(async move {
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
                Ok(Err(e)) | Err(e) => {
                    Err(PostError::ServerError(e.to_string()))
                }
            }
        })
    };

    view! {
        <em>"The world's best content."</em>
        <Suspense fallback=move || view! { <p>"Loading post..."</p> }>
            <ErrorBoundary fallback=|errors| {
                #[cfg(feature = "ssr")]
                expect_context::<leptos_axum::ResponseOptions>()
                    .set_status(http::StatusCode::NOT_FOUND);
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
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostError {
    #[error("Invalid post ID.")]
    InvalidId,
    #[error("Post not found.")]
    PostNotFound,
    #[error("Server error: {0}.")]
    ServerError(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    slug: String,
    title: String,
    content: String,
}

#[server]
pub async fn list_slugs() -> Result<Vec<String>, ServerFnError> {
    use tokio::fs;
    use tokio_stream::{wrappers::ReadDirStream, StreamExt};

    let files = ReadDirStream::new(fs::read_dir("./posts").await?);
    Ok(files
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let extension = path.extension()?;
            if extension != "md" {
                return None;
            }

            let slug = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .replace(".md", "");
            Some(slug)
        })
        .collect()
        .await)
}

#[server]
pub async fn list_posts() -> Result<Vec<Post>, ServerFnError> {
    println!("calling list_posts");

    use futures::TryStreamExt;
    use tokio::fs;
    use tokio_stream::wrappers::ReadDirStream;

    let files = ReadDirStream::new(fs::read_dir("./posts").await?);
    files
        .try_filter_map(|entry| async move {
            let path = entry.path();
            if !path.is_file() {
                return Ok(None);
            }
            let Some(extension) = path.extension() else {
                return Ok(None);
            };
            if extension != "md" {
                return Ok(None);
            }

            let slug = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .replace(".md", "");
            let content = fs::read_to_string(path).await?;
            // world's worst Markdown frontmatter parser
            let title = content.lines().next().unwrap().replace("# ", "");

            Ok(Some(Post {
                slug,
                title,
                content,
            }))
        })
        .try_collect()
        .await
        .map_err(ServerFnError::from)
}

#[server]
pub async fn get_post(slug: String) -> Result<Option<Post>, ServerFnError> {
    println!("reading ./posts/{slug}.md");
    let content =
        tokio::fs::read_to_string(&format!("./posts/{slug}.md")).await?;
    // world's worst Markdown frontmatter parser
    let title = content.lines().next().unwrap().replace("# ", "");

    Ok(Some(Post {
        slug,
        title,
        content,
    }))
}

#[allow(unused)] // path is not used in non-SSR
fn watch_path(path: &Path) -> impl Stream<Item = ()> {
    #[allow(unused)]
    let (mut tx, rx) = mpsc::channel(0);

    #[cfg(feature = "ssr")]
    {
        use notify::{RecursiveMode, Watcher};

        let mut watcher =
            notify::recommended_watcher(move |res: Result<_, _>| {
                if res.is_ok() {
                    // if this fails, it's because the buffer is full
                    // this means we've already notified before it's regenerated,
                    // so this page will be queued for regeneration already
                    _ = tx.try_send(());
                }
            })
            .expect("could not create watcher");

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .expect("could not watch path");

        // we want this to run as long as the server is alive
        std::mem::forget(watcher);
    }

    rx
}
