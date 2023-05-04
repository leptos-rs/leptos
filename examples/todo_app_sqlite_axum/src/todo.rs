use crate::error_template::ErrorTemplate;
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use sqlx::{Connection, SqliteConnection};
        // use http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode};

        pub async fn db() -> Result<SqliteConnection, ServerFnError> {
            SqliteConnection::connect("sqlite:Todos.db").await.map_err(|e| ServerFnError::ServerError(e.to_string()))
        }

        pub fn register_server_functions() {
            _ = GetTodos::register();
            _ = AddTodo::register();
            _ = DeleteTodo::register();
            _ = FormDataHandler::register();
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
pub async fn get_todos(cx: Scope) -> Result<Vec<Todo>, ServerFnError> {
    // this is just an example of how to access server context injected in the handlers
    // http::Request doesn't implement Clone, so more work will be needed to do use_context() on this
    let req_parts = use_context::<leptos_axum::RequestParts>(cx);

    if let Some(req_parts) = req_parts {
        println!("Uri = {:?}", req_parts.uri);
    }

    use futures::TryStreamExt;

    let mut conn = db().await?;

    let mut todos = Vec::new();
    let mut rows =
        sqlx::query_as::<_, Todo>("SELECT * FROM todos").fetch(&mut conn);
    while let Some(row) = rows
        .try_next()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
    {
        todos.push(row);
    }

    // Add a random header(because why not)
    // let mut res_headers = HeaderMap::new();
    // res_headers.insert(SET_COOKIE, HeaderValue::from_str("fizz=buzz").unwrap());

    // let res_parts = leptos_axum::ResponseParts {
    //     headers: res_headers,
    //     status: Some(StatusCode::IM_A_TEAPOT),
    // };

    // let res_options_outer = use_context::<leptos_axum::ResponseOptions>(cx);
    // if let Some(res_options) = res_options_outer {
    //     res_options.overwrite(res_parts).await;
    // }

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
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

#[server(DeleteTodo, "/api")]
pub async fn delete_todo(id: u16) -> Result<(), ServerFnError> {
    let mut conn = db().await?;

    sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&mut conn)
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FormData {
    hi: String,
}

#[server(FormDataHandler, "/api")]
pub async fn form_data(cx: Scope) -> Result<FormData, ServerFnError> {
    use axum::extract::FromRequest;

    let req = use_context::<leptos_axum::LeptosRequest<axum::body::Body>>(cx)
        .and_then(|req| req.take_request())
        .unwrap();
    if req.method() == http::Method::POST {
        let form = axum::Form::from_request(req, &())
            .await
            .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
        Ok(form.0)
    } else {
        Err(ServerFnError::ServerError(
            "wrong form fields submitted".to_string(),
        ))
    }
}

#[component]
pub fn TodoApp(cx: Scope) -> impl IntoView {
    //let id = use_context::<String>(cx);
    provide_meta_context(cx);
    view! {
        cx,
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/todo_app_sqlite_axum.css"/>
        <Router>
            <header>
                <h1>"My Tasks"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! {
                        cx,
                        <ErrorBoundary fallback=|cx, errors| view!{cx, <ErrorTemplate errors=errors/>}>
                            <Todos/>
                        </ErrorBoundary>
                    }/> //Route
                    <Route path="weird" methods=&[Method::Get, Method::Post]
                        ssr=SsrMode::Async
                        view=|cx| {
                            let res = create_resource(cx, || (), move |_| async move {
                                form_data(cx).await
                            });
                            view! { cx,
                                <Suspense fallback=|| ()>
                                    <pre>
                                        {move || {
                                            res.with(cx, |body| format!("{body:#?}"))
                                        }}
                                    </pre>
                                </Suspense>
                            }
                        }
                    />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Todos(cx: Scope) -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>(cx);
    let delete_todo = create_server_action::<DeleteTodo>(cx);
    let submissions = add_todo.submissions();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(
        cx,
        move || (add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(cx),
    );

    view! {
        cx,
        <form method="POST" action="/weird">
            <input type="text" name="hi" value="John"/>
            <input type="submit"/>
        </form>
        <div>
            <MultiActionForm action=add_todo>
                <label>
                    "Add a Todo"
                    <input type="text" name="title"/>
                </label>
                <input type="submit" value="Add"/>
            </MultiActionForm>
            <Transition fallback=move || view! {cx, <p>"Loading..."</p> }>
                {move || {
                    let existing_todos = {
                        move || {
                            todos.read(cx)
                                .map(move |todos| match todos {
                                    Err(e) => {
                                        view! { cx, <pre class="error">"Server Error: " {e.to_string()}</pre>}.into_view(cx)
                                    }
                                    Ok(todos) => {
                                        if todos.is_empty() {
                                            view! { cx, <p>"No tasks were found."</p> }.into_view(cx)
                                        } else {
                                            todos
                                                .into_iter()
                                                .map(move |todo| {
                                                    view! {
                                                        cx,
                                                        <li>
                                                            {todo.title}
                                                            <ActionForm action=delete_todo>
                                                                <input type="hidden" name="id" value={todo.id}/>
                                                                <input type="submit" value="X"/>
                                                            </ActionForm>
                                                        </li>
                                                    }
                                                })
                                                .collect_view(cx)
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
                                cx,
                                <li class="pending">{move || submission.input.get().map(|data| data.title) }</li>
                            }
                        })
                        .collect_view(cx)
                    };

                    view! {
                        cx,
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
