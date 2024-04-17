use super::*;
// An user can post a post. Technically all server functions are POST, so this is a Post Post Post.
#[tracing::instrument(ret)]
#[server]
pub async fn post_post(content: String) -> Result<(), ServerFnError> {
    use crate::database_calls::{create_post, create_post_permissions, PostPermission};

    let pool = leptos_axum::extract::<axum::Extension<sqlx::SqlitePool>>()
        .await?
        .0;
    let user_id = leptos_axum::extract::<crate::auth::extractors::ExtractUserRow>()
        .await?
        .0
        .user_id;
    let PostData { post_id, .. } = create_post(&pool, &user_id, &content).await?;
    create_post_permissions(&pool, &post_id, &user_id, PostPermission::new_full()).await?;
    Ok(())
}

#[component]
pub fn CreatePost() -> impl IntoView {
    let post_post = Action::<PostPost, _>::server();
    view! {
        <ActionForm action=post_post>
            <textarea type="text" name="content" id=ids::POST_POST_TEXT_AREA_ID/>
            <input type="submit" value="Post Post" id=ids::POST_POST_SUBMIT_ID/>
        </ActionForm>
        <Suspense fallback=move||view!{}>
        <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
            { move || post_post.value().get()}
        </ErrorBoundary>
        </Suspense>
    }
}
