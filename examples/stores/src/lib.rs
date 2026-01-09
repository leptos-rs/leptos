use chrono::{Local, NaiveDate};
use leptos::{logging::warn, prelude::*};
use reactive_stores::{Field, Patch, Store};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// ID starts higher than 0 because we have a few starting todos by default
static NEXT_ID: AtomicUsize = AtomicUsize::new(3);

#[derive(Debug, Store, Serialize, Deserialize)]
struct Todos {
    /// Current user.
    user: User,
    /// Vector storage of a collection of todo's.
    #[store(key: usize = |todo| todo.id)]
    todos: Vec<Todo>,
    /// User names to todo IDs.
    #[store(key: Arc<String> = |(name, _)| name.clone())]
    completed: BTreeMap<Arc<String>, usize>,
}

impl Todos {
    fn data() -> Self {
        Self {
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
            completed: Default::default(),
        }
    }
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
impl Todo {
    pub fn new(label: impl ToString) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            label: label.to_string(),
            status: Status::Pending,
        }
    }
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

#[component]
pub fn App() -> impl IntoView {
    let store = Store::new(Todos::data());

    let input_ref = NodeRef::new();

    view! {
        <p>"Hello, " {move || store.user().name().get()}</p>
        <UserForm user=store.user() />
        <UserAchievements store=store.clone() />
        <hr />
        <form on:submit=move |ev| {
            ev.prevent_default();
            store.todos().write().push(Todo::new(input_ref.get().unwrap().value()));
        }>
            <label>"Add a Todo" <input type="text" node_ref=input_ref /></label>
            <input type="submit" />
        </form>
        <ol>
            // because `todos` is a keyed field, `store.todos()` returns a struct that
            // directly implements IntoIterator, so we can use it in <For/> and
            // it will manage reactivity for the store fields correctly
            <For each=move || store.todos() key=|row| row.id().get() let:todo>
                <TodoRow store todo />
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
                "Name" <input type="text" name="name" prop:value=move || user.name().get() />
            </label>
            <label>
                "Email" <input type="email" name="email" prop:value=move || user.email().get() />
            </label>
            <input type="submit" />
        </form>
    }
}

#[component]
fn UserAchievements(store: Store<Todos>) -> impl IntoView {
    let completed = Memo::new(move |_| {
        store
            .completed()
            .at_key(store.user().name().get().into())
            .try_get()
            .unwrap_or_default()
            .to_string()
    });
    view! {
        <div>"You completed: " {completed} " tasks"</div>
        <ul>
            <For each=move || store.completed() key=|row| row.key() let:achievement>
                <div>{achievement.key().to_uppercase()}: {move || achievement.get()}</div>
            </For>
        </ul>
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
            if status.done() { "line-through" } else { Default::default() }
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
                let was_undone = !status.done();
                status.write().next_step();
                if was_undone && status.done() {
                    let name = store.user().name().get();
                    store
                        .completed()
                        .update(|completed| *completed.entry(name.into()).or_default() += 1)
                }
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
