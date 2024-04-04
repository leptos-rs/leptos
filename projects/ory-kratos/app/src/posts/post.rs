use self::posts_page::PostData;

use super::*;

// This is the post, contains all other functionality.
#[component]
pub fn Post(post: PostData) -> impl IntoView {
    let PostData {
        post_id, content, ..
    } = post;
    view! {
        <div>{content}</div>
        <AddEditor post_id=post_id.clone()/>
        <EditPost post_id=post_id.clone()/>
    }
}

// Only the owner can add an an editor.
#[tracing::instrument(ret)]
#[server]
pub async fn server_add_editor(post_id: String, email: String) -> Result<(), ServerFnError> {
    use crate::database_calls::{read_user_by_email, update_post_permission, PostPermission};

    let pool: sqlx::Pool<sqlx::Sqlite> =
        leptos_axum::extract::<axum::Extension<sqlx::SqlitePool>>()
            .await?
            .0;

    let user_id = leptos_axum::extract::<crate::auth::extractors::ExtractUserRow>()
        .await?
        .0
        .user_id;

    let caller_permissions = PostPermission::from_db_call(&pool, &user_id, &post_id).await?;

    caller_permissions.is_full()?;

    // get other id
    let user_id = read_user_by_email(&pool, &email).await?.user_id;

    // make an idempotent update to the other users permissions;
    let mut permissions = PostPermission::from_db_call(&pool, &post_id, &user_id).await?;
    permissions.write = true;
    permissions.read = true;

    update_post_permission(&pool, &post_id, &user_id, permissions).await?;

    Ok(())
}

#[component]
pub fn AddEditor(post_id: String) -> impl IntoView {
    let add_editor = Action::<ServerAddEditor, _>::server();
    view! {
        <ActionForm action=add_editor>
            <label value="Add Editor Email">
            <input type="text"  name="email" id=ids::POST_ADD_EDITOR_INPUT_ID/>
            <input type="hidden" name="post_id" value=post_id/>
            </label>
            <input type="submit" id=ids::POST_ADD_EDITOR_SUBMIT_ID/>
        </ActionForm>
        <Suspense fallback=||view!{}>
            <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
            { move || add_editor.value().get()}
            </ErrorBoundary>
        </Suspense>
    }
}

// Only the owner and editors can edit a post.
#[tracing::instrument(ret)]
#[server]
pub async fn server_edit_post(post_id: String, content: String) -> Result<(), ServerFnError> {
    let pool: sqlx::Pool<sqlx::Sqlite> =
        leptos_axum::extract::<axum::Extension<sqlx::SqlitePool>>()
            .await?
            .0;

    let user_id = leptos_axum::extract::<crate::auth::extractors::ExtractUserRow>()
        .await?
        .0
        .user_id;

    crate::database_calls::edit_post(&pool, &post_id, &content, &user_id).await?;

    Ok(())
}

#[component]
pub fn EditPost(post_id: String) -> impl IntoView {
    let edit_post = Action::<ServerEditPost, _>::server();
    view! {
        <ActionForm action=edit_post>
            <label value="New Content:">
            <textarea name="content" id=ids::POST_EDIT_TEXT_AREA_ID/>
            <input type="hidden" name="post_id" value=post_id/>
            </label>
            <input type="submit" id=ids::POST_EDIT_SUBMIT_ID/>
        </ActionForm>
        <Suspense fallback=||view!{}>
            <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
                { move || edit_post.value().get()}
            </ErrorBoundary>
        </Suspense>
    }
}
