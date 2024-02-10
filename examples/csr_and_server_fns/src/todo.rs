use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Todo {
    id: u16,
    title: String,
    completed: bool,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    pub use actix_web::HttpRequest;
    pub use leptos::ServerFnError;
    pub use sqlx::{Connection, SqliteConnection};

    pub async fn db() -> Result<SqliteConnection, ServerFnError> {
        Ok(SqliteConnection::connect("sqlite:Todos.db").await?)
    }
}

/// This is an example of a server function using an alternative CBOR encoding. Both the function arguments being sent
/// to the server and the server response will be encoded with CBOR. Good for binary data that doesn't encode well via the default methods
#[server(encoding = "Cbor")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    use self::ssr::*;

    // this is just an example of how to access server context injected in the handlers
    let req = use_context::<HttpRequest>();

    if let Some(req) = req {
        println!("req.path = {:#?}", req.path());
    }
    use futures::TryStreamExt;

    let mut conn = db().await?;

    let mut todos = Vec::new();
    let mut rows =
        sqlx::query_as::<_, Todo>("SELECT * FROM todos").fetch(&mut conn);
    while let Some(row) = rows.try_next().await? {
        todos.push(row);
    }

    Ok(todos)
}

#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    use self::ssr::*;

    let mut conn = db().await?;

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1250));

    match sqlx::query("INSERT INTO todos (title, completed) VALUES ($1, false)")
        .bind(title)
        .execute(&mut conn)
        .await
    {
        Ok(_row) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

// The struct name and path prefix arguments are optional.
#[server]
pub async fn delete_todo(id: u16) -> Result<(), ServerFnError> {
    use self::ssr::*;

    let mut conn = db().await?;

    Ok(sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&mut conn)
        .await
        .map(|_| ())?)
}

#[component]
pub fn TodoApp() -> impl IntoView {
    provide_meta_context();

    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/todo_app_sqlite.css"/>
        <Router>
            <header>
                <h1>"My Tasks"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=Todos/>
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
            <MultiActionForm
                // we can handle client-side validation in the on:submit event
                // leptos_router implements a `FromFormData` trait that lets you
                // parse deserializable types from form data and check them
                on:submit=move |ev| {
                    let data = AddTodo::from_event(&ev).expect("to parse form data");
                    // silly example of validation: if the todo is "nope!", nope it
                    if data.title == "nope!" {
                        // ev.prevent_default() will prevent form submission
                        ev.prevent_default();
                    }
                }
                action=add_todo
            >
                <label>
                    "Add a Todo"
                    <input type="text" name="title"/>
                </label>
                <input type="submit" value="Add"/>
            </MultiActionForm>
            <Transition fallback=move || view! {<p>"Loading..."</p> }>
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
            </Transition>
        </div>
    }
}
