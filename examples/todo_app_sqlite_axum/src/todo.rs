use crate::error_template::ErrorTemplate;
use leptos::context::use_context;
use leptos::server::{Resource, ServerAction};
use leptos::tachys::either::Either;
use leptos::{
    component, server, suspend, view, ActionForm, ErrorBoundary, IntoView,
};
use leptos::{prelude::*, Suspense, Transition};
use serde::{Deserialize, Serialize};
use server_fn::codec::SerdeLite;
use server_fn::ServerFnError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Todo {
    id: u16,
    title: String,
    completed: bool,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    // use http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode};
    use leptos::server_fn::ServerFnError;
    use sqlx::{Connection, SqliteConnection};

    pub async fn db() -> Result<SqliteConnection, ServerFnError> {
        Ok(SqliteConnection::connect("sqlite:Todos.db").await?)
    }
}

#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    use self::ssr::*;
    use http::request::Parts;

    // this is just an example of how to access server context injected in the handlers
    let req_parts = use_context::<Parts>();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    if let Some(req_parts) = req_parts {
        println!("Uri = {:?}", req_parts.uri);
    }

    use futures::TryStreamExt;

    let mut conn = db().await?;

    let mut todos = Vec::new();
    let mut rows =
        sqlx::query_as::<_, Todo>("SELECT * FROM todos").fetch(&mut conn);
    while let Some(row) = rows.try_next().await? {
        todos.push(row);
    }

    // Lines below show how to set status code and headers on the response
    // let resp = expect_context::<ResponseOptions>();
    // resp.set_status(StatusCode::IM_A_TEAPOT);
    // resp.insert_header(SET_COOKIE, HeaderValue::from_str("fizz=buzz").unwrap());

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

#[server(output = SerdeLite)]
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
    view! {
        <header>
            <h1>"My Tasks"</h1>
        </header>
        <main>
            <Todos/>
        </main>
    }
}

#[component]
pub fn Todos() -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>();
    let delete_todo = ServerAction::<DeleteTodo>::new();
    let submissions = add_todo.submissions();
    let delete_todo = ServerAction::<DeleteTodo>::new();
    //let submissions = add_todo.submissions();

    // list of todos is loaded from the server in reaction to changes
    let todos = Resource::new_serde(
        move || (delete_todo.version().get()), //(add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(),
    );

    view! {
        <div>
            <Transition fallback=move || view! { <p>"Loading..."</p> }>
                <ErrorBoundary fallback=|errors| view! { <ErrorTemplate errors/> }>
                    // {existing_todos}
                    <ul>
                        {move || {
                            async move {
                                todos
                                    .await
                                    .map(|todos| {
                                        if todos.is_empty() {
                                            Either::Left(view! { <p>"No tasks were found."</p> })
                                        } else {
                                            Either::Right(
                                                todos
                                                    .into_iter()
                                                    .map(move |todo| {
                                                        view! {
                                                            <li>
                                                                {todo.title} <ActionForm action=delete_todo>
                                                                    <input type="hidden" name="id" value=todo.id/>
                                                                    <input type="submit" value="X"/>
                                                                </ActionForm>
                                                            </li>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>(),
                                            )
                                        }
                                    })
                            }
                                .wait()
                        }}

                    // {pending_todos}
                    </ul>
                </ErrorBoundary>
            </Transition>
        </div>
    }
}
