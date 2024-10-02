use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{Local, NaiveDate};
use leptos::prelude::*;
use reactive_stores::{Field, Patch, Store};
use reactive_stores_macro::{Patch, Store};
use serde::{Deserialize, Serialize};

// ID starts higher than 0 because we have a few starting todos by default
static NEXT_ID: AtomicUsize = AtomicUsize::new(3);

#[derive(Debug, Store, Serialize, Deserialize)]
struct Todos {
    user: User,
    #[store(key: usize = |todo| todo.id)]
    todos: Vec<Todo>,
}

#[derive(Debug, Store, Patch, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

#[derive(Debug, Store, Serialize, Deserialize)]
struct Todo {
    id: usize,
    label: String,
    status: Status,
}

#[derive(Debug, Default, Clone, Store, Serialize, Deserialize)]
enum Status {
    #[default]
    Pending,
    Scheduled,
    ScheduledFor {
        date: NaiveDate,
    },
    Done,
}

impl Status {
    pub fn next_step(&mut self) {
        *self = match self {
            Status::Pending => Status::ScheduledFor {
                date: Local::now().naive_local().into(),
            },
            Status::Scheduled | Status::ScheduledFor { .. } => Status::Done,
            Status::Done => Status::Done,
        };
    }
}

impl Todo {
    pub fn new(label: impl ToString) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            label: label.to_string(),
            status: Status::Pending,
        }
    }
}

fn data() -> Todos {
    Todos {
        user: User {
            name: "Bob".to_string(),
            email: "lawblog@bobloblaw.com".into(),
        },
        todos: vec![
            Todo {
                id: 0,
                label: "Create reactive store".to_string(),
                status: Status::Pending,
            },
            Todo {
                id: 1,
                label: "???".to_string(),
                status: Status::Pending,
            },
            Todo {
                id: 2,
                label: "Profit".to_string(),
                status: Status::Pending,
            },
        ],
    }
}

#[component]
pub fn App() -> impl IntoView {
    let store = Store::new(data());

    let input_ref = NodeRef::new();

    view! {
        <p>"Hello, " {move || store.user().name().get()}</p>
        <UserForm user=store.user()/>
        <hr/>
        <form on:submit=move |ev| {
            ev.prevent_default();
            store.todos().write().push(Todo::new(input_ref.get().unwrap().value()));
        }>
            <label>"Add a Todo" <input type="text" node_ref=input_ref/></label>
            <input type="submit"/>
        </form>
        <ol>
            // because `todos` is a keyed field, `store.todos()` returns a struct that
            // directly implements IntoIterator, so we can use it in <For/> and
            // it will manage reactivity for the store fields correctly
            <For
                each=move || {
                    leptos::logging::log!("RERUNNING FOR CALCULATION");
                    store.todos()
                }

                key=|row| row.id().get()
                let:todo
            >
                <TodoRow store todo/>
            </For>

        </ol>
        <pre>{move || serde_json::to_string_pretty(&*store.read())}</pre>
    }
}

#[component]
fn UserForm(#[prop(into)] user: Field<User>) -> impl IntoView {
    let error = RwSignal::new(None);

    view! {
        {move || error.get().map(|n| view! { <p>{n}</p> })}
        <form on:submit:target=move |ev| {
            ev.prevent_default();
            match User::from_event(&ev) {
                Ok(new_user) => {
                    error.set(None);
                    user.patch(new_user);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }>
            <label>
                "Name" <input type="text" name="name" prop:value=move || user.name().get()/>
            </label>
            <label>
                "Email" <input type="email" name="email" prop:value=move || user.email().get()/>
            </label>
            <input type="submit"/>
        </form>
    }
}

#[component]
fn TodoRow(
    store: Store<Todos>,
    #[prop(into)] todo: Field<Todo>,
) -> impl IntoView {
    let status = todo.status();
    let title = todo.label();

    let editing = RwSignal::new(true);

    view! {
        <li style:text-decoration=move || {
            status.done().then_some("line-through").unwrap_or_default()
        }>

            <p
                class:hidden=move || editing.get()
                on:click=move |_| {
                    editing.update(|n| *n = !*n);
                }
            >

                {move || title.get()}
            </p>
            <input
                class:hidden=move || !(editing.get())
                type="text"
                prop:value=move || title.get()
                on:change=move |ev| {
                    title.set(event_target_value(&ev));
                }
            />

            <button on:click=move |_| {
                status.write().next_step()
            }>
                {move || {
                    if todo.status().done() {
                        "Done"
                    } else if status.scheduled() || status.scheduled_for() {
                        "Scheduled"
                    } else {
                        "Pending"
                    }
                }}

            </button>

            <button on:click=move |_| {
                let id = todo.id().get();
                store.todos().write().retain(|todo| todo.id != id);
            }>"X"</button>
            <input
                type="date"
                prop:value=move || {
                    todo.status().scheduled_for_date().map(|n| n.get().to_string())
                }

                class:hidden=move || !todo.status().scheduled_for()
                on:change:target=move |ev| {
                    if let Some(date) = todo.status().scheduled_for_date() {
                        let value = ev.target().value();
                        match NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                            Ok(new_date) => {
                                date.set(new_date);
                            }
                            Err(e) => warn!("{e}"),
                        }
                    }
                }
            />

        </li>
    }
}
