use leptos::ServerFnError;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, FromRow};

// This will just map into ServerFnError when we call it in our serverfunctions with ? error handling
use sqlx::Error;

use crate::posts_page::PostData;
#[tracing::instrument(err)]
pub async fn create_user(
    pool: &SqlitePool,
    identity_id: &String,
    email: &String,
) -> Result<(), Error> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO users (user_id,identity_id,email) VALUES (?,?,?)",
        id,
        identity_id,
        email
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns the POST ROW
#[tracing::instrument(ret)]
pub async fn create_post(
    pool: &SqlitePool,
    user_id: &String,
    content: &String,
) -> Result<PostData, Error> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query_as!(
        PostData,
        "INSERT INTO posts (post_id,user_id,content) VALUES (?,?,?) RETURNING *",
        id,
        user_id,
        content
    )
    .fetch_one(pool)
    .await
}
#[tracing::instrument(ret)]

pub async fn edit_post(
    pool: &SqlitePool,
    post_id: &String,
    content: &String,
    user_id: &String,
) -> Result<(), Error> {
    sqlx::query!(
        "
    UPDATE posts
    SET content = ?
    WHERE post_id = ?
    AND EXISTS (
        SELECT 1
        FROM post_permissions
        WHERE post_permissions.post_id = posts.post_id
        AND post_permissions.user_id = ?
        AND post_permissions.write = TRUE
    )",
        content,
        post_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
#[tracing::instrument(ret)]

pub async fn delete_post(pool: &SqlitePool, post_id: &String) -> Result<(), Error> {
    sqlx::query!("DELETE FROM posts where post_id = ?", post_id)
        .execute(pool)
        .await?;
    Ok(())
}
#[tracing::instrument(ret)]

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<UserRow>, Error> {
    sqlx::query_as::<_, UserRow>("SELECT user_id, identity_id FROM users")
        .fetch_all(pool)
        .await
}
#[tracing::instrument(ret)]

pub async fn read_user(pool: &SqlitePool, user_id: &String) -> Result<UserRow, Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
}
#[tracing::instrument(ret)]
pub async fn read_user_by_identity_id(
    pool: &SqlitePool,
    identity_id: &String,
) -> Result<UserRow, Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE identity_id = ?")
        .bind(identity_id)
        .fetch_one(pool)
        .await
}
#[tracing::instrument(ret)]

pub async fn read_user_by_email(pool: &SqlitePool, email: &String) -> Result<UserRow, Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_one(pool)
        .await
}
#[tracing::instrument(ret)]

pub async fn list_posts(pool: &SqlitePool, user_id: &String) -> Result<Vec<PostData>, Error> {
    sqlx::query_as::<_, PostData>(
        "
    SELECT posts.*
    FROM posts
    JOIN post_permissions ON posts.post_id = post_permissions.post_id 
        AND post_permissions.user_id = ?
    WHERE post_permissions.read = TRUE
    ",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

#[tracing::instrument(ret)]

pub async fn update_post_permission(
    pool: &SqlitePool,
    post_id: &String,
    user_id: &String,
    PostPermission {
        read,
        write,
        delete,
    }: PostPermission,
) -> Result<(), Error> {
    sqlx::query!(
        "
        INSERT INTO post_permissions (post_id, user_id, read, write, `delete`)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (post_id, user_id) DO UPDATE SET
        read = excluded.read,
        write = excluded.write,
        `delete` = excluded.`delete`;
        ",
        post_id,
        user_id,
        read,
        write,
        delete
    )
    .execute(pool)
    .await?;

    Ok(())
}
#[tracing::instrument(ret)]
pub async fn create_post_permissions(
    pool: &SqlitePool,
    post_id: &String,
    user_id: &String,
    PostPermission {
        read,
        write,
        delete,
    }: PostPermission,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO post_permissions (post_id,user_id,read,write,`delete`) VALUES (?,?,?,?,?)",
        post_id,
        user_id,
        read,
        write,
        delete
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct PostPermission {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
}

impl PostPermission {
    #[tracing::instrument(ret)]
    pub async fn from_db_call(
        pool: &SqlitePool,
        user_id: &String,
        post_id: &String,
    ) -> Result<Self, Error> {
        if let Ok(row) = sqlx::query_as!(
            PostPermissionRow,
            "SELECT * FROM post_permissions WHERE post_id = ? AND user_id = ?",
            post_id,
            user_id
        )
        .fetch_one(pool)
        .await
        {
            Ok(Self::from(row))
        } else {
            Ok(Self::default())
        }
    }

    pub fn new_full() -> Self {
        Self {
            read: true,
            write: true,
            delete: true,
        }
    }

    pub fn is_full(&self) -> Result<(), ServerFnError> {
        if &Self::new_full() != self {
            Err(ServerFnError::new("Unauthorized, not full permissions. "))
        } else {
            Ok(())
        }
    }
    pub fn can_read(&self) -> Result<(), ServerFnError> {
        if !self.read {
            Err(ServerFnError::new("Unauthorized to read"))
        } else {
            Ok(())
        }
    }
    pub fn can_write(&self) -> Result<(), ServerFnError> {
        if !self.write {
            Err(ServerFnError::new("Unauthorized to write"))
        } else {
            Ok(())
        }
    }
    pub fn can_delete(&self) -> Result<(), ServerFnError> {
        if !self.delete {
            Err(ServerFnError::new("Unauthorized to delete"))
        } else {
            Ok(())
        }
    }
}

impl From<PostPermissionRow> for PostPermission {
    fn from(value: PostPermissionRow) -> Self {
        Self {
            read: value.read,
            write: value.write,
            delete: value.delete,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, FromRow)]
pub struct PostPermissionRow {
    pub post_id: String,
    pub user_id: String,
    pub read: bool,
    pub write: bool,
    pub delete: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, FromRow)]
pub struct UserRow {
    pub user_id: String,
    pub identity_id: String,
    pub email: String,
}
