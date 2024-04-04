use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct PostData {
    pub post_id: String,
    pub user_id: String,
    pub content: String,
}
impl IntoView for PostData {
    fn into_view(self) -> View {
        view! {<Post post=self/>}
    }
}

#[tracing::instrument(ret)]
#[server]
pub async fn get_post_list() -> Result<Vec<PostData>, ServerFnError> {
    use crate::database_calls::list_posts;

    let pool = leptos_axum::extract::<axum::Extension<sqlx::SqlitePool>>()
        .await?
        .0;

    let user_id = leptos_axum::extract::<crate::auth::extractors::ExtractUserRow>()
        .await?
        .0
        .user_id;

    Ok(list_posts(&pool, &user_id).await?)
}

#[component]
pub fn PostPage() -> impl IntoView {
    view! {
        <PostsList/>
        <CreatePost/>
    }
}

#[component]
pub fn PostsList() -> impl IntoView {
    let list_posts = Action::<GetPostList, _>::server();

    view! {
        <button on:click=move|_|list_posts.dispatch(GetPostList{}) id=ids::POST_SHOW_LIST_BUTTON_ID>Show List</button>
        <Suspense fallback=||"Post list loading...".into_view()>
        <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
            {
                move || list_posts.value().get().map(|resp|
                    match resp {
                        Ok(list) => view!{
                            <For
                            each=move || list.clone()
                            key=|_| uuid::Uuid::new_v4()
                            children=move |post: PostData| {
                              post.into_view()
                            }
                          />
                        }.into_view(),
                        err => err.into_view()
                    })
            }
        </ErrorBoundary>
        </Suspense>
    }
}
