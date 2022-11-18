use cfg_if::cfg_if;
use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use sqlx::{Connection, SqliteConnection};

        pub async fn db() -> Result<SqliteConnection, ServerFnError> {
            Ok(SqliteConnection::connect("sqlite:Todos.db").await.map_err(|e| ServerFnError::ServerError(e.to_string()))?)
        }

        pub fn register_server_functions() {
            GetTodos::register();
            AddTodo::register();
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
        pub struct Todo {
            id: u16,
            title: String,
            completed: bool,
        }
    } else {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct Todo {
            id: u16,
            title: String,
            completed: bool,
        }
    }
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    use futures::TryStreamExt;

    let mut conn = db().await?;

    let mut todos = Vec::new();
    let mut rows = sqlx::query_as::<_, Todo>("SELECT * FROM todos").fetch(&mut conn);
    while let Some(row) = rows
        .try_next()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
    {
        todos.push(row);
    }

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<u16, ServerFnError> {
    use futures::TryStreamExt;

    let mut conn = db().await?;

    match sqlx::query("INSERT INTO todos (title, completed) VALUES ($1, false)")
        .bind(title)
        .execute(&mut conn)
        .await
    {
        Ok(row) => Ok(0),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[component]
pub fn TodoApp(cx: Scope) -> Element {
    view! {
        cx,
        <div>
            <Router>
                <header>
                    <h1>"My Tasks"</h1>
                </header>
                <main>
                    <Routes>
                        <Route path="" element=|cx| view! {
                            cx,
                            <Todos/>
                        }/>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}

#[component]
pub fn Todos(cx: Scope) -> Element {
    let add_todo = create_server_multi_action::<AddTodo>(cx);
    let add_changed = add_todo.version;

    let todos = create_resource(cx, move || add_changed(), |_| get_todos());
    let todos_view = move || {
        todos.read().map(|todos| match todos {
            Err(e) => view! { cx, <pre class="error">"Server Error: " {e.to_string()}</pre>},
            Ok(todos) => {
                if todos.is_empty() {
                    view! { cx, <p>"No tasks were found."</p> }
                } else {
                    let todos = todos
                        .into_iter()
                        .map(|todo| view! { cx, <li>{todo.title}</li> })
                        .collect::<Vec<_>>();
                    view! {
                        cx,
                        <ul>{todos}</ul>
                    }
                }
            }
        })
    };

    view! {
        cx,
        <div>
            <MultiActionForm action=add_todo>
                <label>
                    "Add a Todo"
                    <input type="text" name="title"/>
                </label>
                <input type="submit" value="Add"/>
            </MultiActionForm>
            {todos_view}
        </div>
    }
}
