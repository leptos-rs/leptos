use crate::{auth::*, error_template::ErrorTemplate};
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    id: u32,
    user: Option<User>,
    title: String,
    created_at: String,
    completed: bool,
}

cfg_if! {
if #[cfg(feature = "ssr")] {

    use sqlx::SqlitePool;

    pub fn pool() -> Result<SqlitePool, ServerFnError> {
       use_context::<SqlitePool>()
            .ok_or_else(|| ServerFnError::ServerError("Pool missing.".into()))
    }

    pub fn auth() -> Result<AuthSession, ServerFnError> {
        use_context::<AuthSession>()
            .ok_or_else(|| ServerFnError::ServerError("Auth session missing.".into()))
    }

    #[derive(sqlx::FromRow, Clone)]
    pub struct SqlTodo {
        id: u32,
        user_id: i64,
        title: String,
        created_at: String,
        completed: bool,
    }

    impl SqlTodo {
        pub async fn into_todo(self, pool: &SqlitePool) -> Todo {
            Todo {
                id: self.id,
                user: User::get(self.user_id, pool).await,
                title: self.title,
                created_at: self.created_at,
                completed: self.completed,
            }
        }
    }
}
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    use futures::TryStreamExt;

    let pool = pool()?;

    let mut todos = Vec::new();
    let mut rows =
        sqlx::query_as::<_, SqlTodo>("SELECT * FROM todos").fetch(&pool);

    while let Some(row) = rows.try_next().await? {
        todos.push(row);
    }

    // why can't we just have async closures?
    // let mut rows: Vec<Todo> = rows.iter().map(|t| async { t }).collect();

    let mut converted_todos = Vec::with_capacity(todos.len());

    for t in todos {
        let todo = t.into_todo(&pool).await;
        converted_todos.push(todo);
    }

    let todos: Vec<Todo> = converted_todos;

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    let user = get_user().await?;
    let pool = pool()?;

    let id = match user {
        Some(user) => user.id,
        None => -1,
    };

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1250));

    match sqlx::query(
        "INSERT INTO todos (title, user_id, completed) VALUES (?, ?, false)",
    )
    .bind(title)
    .bind(id)
    .execute(&pool)
    .await
    {
        Ok(_row) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

// The struct name and path prefix arguments are optional.
#[server]
pub async fn delete_todo(id: u16) -> Result<(), ServerFnError> {
    let pool = pool()?;

    Ok(sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map(|_| ())?)
}

#[component]
pub fn TodoApp() -> impl IntoView {
    let login = create_server_action::<Login>();
    let logout = create_server_action::<Logout>();
    let signup = create_server_action::<Signup>();

    let user = create_resource(
        move || {
            (
                login.version().get(),
                signup.version().get(),
                logout.version().get(),
            )
        },
        move |_| get_user(),
    );
    provide_meta_context();

    view! {

        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/session_auth_axum.css"/>
        <Router>
            <header>
                <A href="/"><h1>"My Tasks"</h1></A>
                <Transition
                    fallback=move || view! {<span>"Loading..."</span>}
                >
                {move || {
                    user.get().map(|user| match user {
                        Err(e) => view! {
                            <A href="/signup">"Signup"</A>", "
                            <A href="/login">"Login"</A>", "
                            <span>{format!("Login error: {}", e)}</span>
                        }.into_view(),
                        Ok(None) => view! {
                            <A href="/signup">"Signup"</A>", "
                            <A href="/login">"Login"</A>", "
                            <span>"Logged out."</span>
                        }.into_view(),
                        Ok(Some(user)) => view! {
                            <A href="/settings">"Settings"</A>", "
                            <span>{format!("Logged in as: {} ({})", user.username, user.id)}</span>
                        }.into_view()
                    })
                }}
                </Transition>
            </header>
            <hr/>
            <main>
                <Routes>
                    <Route path="" view=Todos/> //Route
                    <Route path="signup" view=move || view! {
                        <Signup action=signup/>
                    }/>
                    <Route path="login" view=move || view! {

                        <Login action=login />
                    }/>
                    <Route path="settings" view=move || view! {

                        <h1>"Settings"</h1>
                        <Logout action=logout />
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>();
    let delete_todo = create_server_action::<DeleteTodo>();
    let submissions = add_todo.submissions();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(
        move || (add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(),
    );

    view! {

        <div>
            <MultiActionForm action=add_todo>
                <label>
                    "Add a Todo"
                    <input type="text" name="title"/>
                </label>
                <input type="submit" value="Add"/>
            </MultiActionForm>
            <Transition fallback=move || view! {<p>"Loading..."</p> }>
                <ErrorBoundary fallback=|errors| view!{ <ErrorTemplate errors=errors/>}>
                    {move || {
                        let existing_todos = {
                            move || {
                                todos.get()
                                    .map(move |todos| match todos {
                                        Err(e) => {
                                            view! { <pre class="error">"Server Error: " {e.to_string()}</pre>}.into_view()
                                        }
                                        Ok(todos) => {
                                            if todos.is_empty() {
                                                view! { <p>"No tasks were found."</p> }.into_view()
                                            } else {
                                                todos
                                                    .into_iter()
                                                    .map(move |todo| {
                                                        view! {

                                                            <li>
                                                                {todo.title}
                                                                ": Created at "
                                                                {todo.created_at}
                                                                " by "
                                                                {
                                                                    todo.user.unwrap_or_default().username
                                                                }
                                                                <ActionForm action=delete_todo>
                                                                    <input type="hidden" name="id" value={todo.id}/>
                                                                    <input type="submit" value="X"/>
                                                                </ActionForm>
                                                            </li>
                                                        }
                                                    })
                                                    .collect_view()
                                            }
                                        }
                                    })
                                    .unwrap_or_default()
                            }
                        };

                        let pending_todos = move || {
                            submissions
                            .get()
                            .into_iter()
                            .filter(|submission| submission.pending().get())
                            .map(|submission| {
                                view! {

                                    <li class="pending">{move || submission.input.get().map(|data| data.title) }</li>
                                }
                            })
                            .collect_view()
                        };

                        view! {

                            <ul>
                                {existing_todos}
                                {pending_todos}
                            </ul>
                        }
                    }
                }
                </ErrorBoundary>
            </Transition>
        </div>
    }
}

#[component]
pub fn Login(
    action: Action<Login, Result<(), ServerFnError>>,
) -> impl IntoView {
    view! {

        <ActionForm action=action>
            <h1>"Log In"</h1>
            <label>
                "User ID:"
                <input type="text" placeholder="User ID" maxlength="32" name="username" class="auth-input" />
            </label>
            <br/>
            <label>
                "Password:"
                <input type="password" placeholder="Password" name="password" class="auth-input" />
            </label>
            <br/>
            <label>
                <input type="checkbox" name="remember" class="auth-input" />
                "Remember me?"
            </label>
            <br/>
            <button type="submit" class="button">"Log In"</button>
        </ActionForm>
    }
}

#[component]
pub fn Signup(
    action: Action<Signup, Result<(), ServerFnError>>,
) -> impl IntoView {
    view! {

        <ActionForm action=action>
            <h1>"Sign Up"</h1>
            <label>
                "User ID:"
                <input type="text" placeholder="User ID" maxlength="32" name="username" class="auth-input" />
            </label>
            <br/>
            <label>
                "Password:"
                <input type="password" placeholder="Password" name="password" class="auth-input" />
            </label>
            <br/>
            <label>
                "Confirm Password:"
                <input type="password" placeholder="Password again" name="password_confirmation" class="auth-input" />
            </label>
            <br/>
            <label>
                "Remember me?"
                <input type="checkbox" name="remember" class="auth-input" />
            </label>

            <br/>
            <button type="submit" class="button">"Sign Up"</button>
        </ActionForm>
    }
}

#[component]
pub fn Logout(
    action: Action<Logout, Result<(), ServerFnError>>,
) -> impl IntoView {
    view! {

        <div id="loginbox">
            <ActionForm action=action>
                <button type="submit" class="button">"Log Out"</button>
            </ActionForm>
        </div>
    }
}
