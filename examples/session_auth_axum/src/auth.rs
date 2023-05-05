use cfg_if::cfg_if;
use leptos::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

cfg_if! {
if #[cfg(feature = "ssr")] {
    use sqlx::SqlitePool;
    use axum_sessions_auth::{SessionSqlitePool, Authentication, HasPermission};
    use bcrypt::{hash, verify, DEFAULT_COST};
    use crate::todo::{pool, auth};
    pub type AuthSession = axum_sessions_auth::AuthSession<User, i64, SessionSqlitePool, SqlitePool>;
}}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub permissions: HashSet<String>,
}

impl Default for User {
    fn default() -> Self {
        let permissions = HashSet::new();

        Self {
            id: -1,
            username: "Guest".into(),
            password: "".into(),
            permissions,
        }
    }
}

cfg_if! {
if #[cfg(feature = "ssr")] {
    use async_trait::async_trait;

    impl User {
        pub async fn get(id: i64, pool: &SqlitePool) -> Option<Self> {
            let sqluser = sqlx::query_as::<_, SqlUser>("SELECT * FROM users WHERE id = ?")
                .bind(id)
                .fetch_one(pool)
                .await
                .ok()?;

            //lets just get all the tokens the user can use, we will only use the full permissions if modifing them.
            let sql_user_perms = sqlx::query_as::<_, SqlPermissionTokens>(
                "SELECT token FROM user_permissions WHERE user_id = ?;",
            )
            .bind(id)
            .fetch_all(pool)
            .await
            .ok()?;

            Some(sqluser.into_user(Some(sql_user_perms)))
        }

        pub async fn get_from_username(name: String, pool: &SqlitePool) -> Option<Self> {
            let sqluser = sqlx::query_as::<_, SqlUser>("SELECT * FROM users WHERE username = ?")
                .bind(name)
                .fetch_one(pool)
                .await
                .ok()?;

            //lets just get all the tokens the user can use, we will only use the full permissions if modifing them.
            let sql_user_perms = sqlx::query_as::<_, SqlPermissionTokens>(
                "SELECT token FROM user_permissions WHERE user_id = ?;",
            )
            .bind(sqluser.id)
            .fetch_all(pool)
            .await
            .ok()?;

            Some(sqluser.into_user(Some(sql_user_perms)))
        }
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlPermissionTokens {
        pub token: String,
    }

    #[async_trait]
    impl Authentication<User, i64, SqlitePool> for User {
        async fn load_user(userid: i64, pool: Option<&SqlitePool>) -> Result<User, anyhow::Error> {
            let pool = pool.unwrap();

            User::get(userid, pool)
                .await
                .ok_or_else(|| anyhow::anyhow!("Cannot get user"))
        }

        fn is_authenticated(&self) -> bool {
            true
        }

        fn is_active(&self) -> bool {
            true
        }

        fn is_anonymous(&self) -> bool {
            false
        }
    }

    #[async_trait]
    impl HasPermission<SqlitePool> for User {
        async fn has(&self, perm: &str, _pool: &Option<&SqlitePool>) -> bool {
            self.permissions.contains(perm)
        }
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlUser {
        pub id: i64,
        pub username: String,
        pub password: String,
    }

    impl SqlUser {
        pub fn into_user(self, sql_user_perms: Option<Vec<SqlPermissionTokens>>) -> User {
            User {
                id: self.id,
                username: self.username,
                password: self.password,
                permissions: if let Some(user_perms) = sql_user_perms {
                    user_perms
                        .into_iter()
                        .map(|x| x.token)
                        .collect::<HashSet<String>>()
                } else {
                    HashSet::<String>::new()
                },
            }
        }
    }
}
}

#[server(Foo, "/api")]
pub async fn foo() -> Result<String, ServerFnError> {
    Ok(String::from("Bar!"))
}

#[server(GetUser, "/api")]
pub async fn get_user(cx: Scope) -> Result<Option<User>, ServerFnError> {
    let auth = auth(cx)?;

    Ok(auth.current_user)
}

#[server(Login, "/api")]
pub async fn login(
    cx: Scope,
    username: String,
    password: String,
    remember: Option<String>,
) -> Result<(), ServerFnError> {
    let pool = pool(cx)?;
    let auth = auth(cx)?;

    let user: User = User::get_from_username(username, &pool)
        .await
        .ok_or("User does not exist.")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    match verify(password, &user.password)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
    {
        true => {
            auth.login_user(user.id);
            auth.remember_user(remember.is_some());
            leptos_axum::redirect(cx, "/");
            Ok(())
        }
        false => Err(ServerFnError::ServerError(
            "Password does not match.".to_string(),
        )),
    }
}

#[server(Signup, "/api")]
pub async fn signup(
    cx: Scope,
    username: String,
    password: String,
    password_confirmation: String,
    remember: Option<String>,
) -> Result<(), ServerFnError> {
    let pool = pool(cx)?;
    let auth = auth(cx)?;

    if password != password_confirmation {
        return Err(ServerFnError::ServerError(
            "Passwords did not match.".to_string(),
        ));
    }

    let password_hashed = hash(password, DEFAULT_COST).unwrap();

    sqlx::query("INSERT INTO users (username, password) VALUES (?,?)")
        .bind(username.clone())
        .bind(password_hashed)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    let user = User::get_from_username(username, &pool)
        .await
        .ok_or("Signup failed: User does not exist.")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    auth.login_user(user.id);
    auth.remember_user(remember.is_some());

    leptos_axum::redirect(cx, "/");

    Ok(())
}

#[server(Logout, "/api")]
pub async fn logout(cx: Scope) -> Result<(), ServerFnError> {
    let auth = auth(cx)?;

    auth.logout_user();
    leptos_axum::redirect(cx, "/");

    Ok(())
}
