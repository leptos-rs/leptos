use chrono::{Local, NaiveDate};
use leptos::{logging::warn, prelude::*};
use reactive_stores::{Field, Store};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct TodoKey;
}

#[derive(Debug, Store, Default, Serialize, Deserialize)]
pub struct Data {
    /// Todos per user.
    #[store(key: TodoKey = |(k,_)| k)]
    todos: SlotMap<TodoKey, Todo>,
}

#[derive(Clone, Debug, Store, Serialize, Deserialize)]
struct Todo {
    id: TodoKey,
    label: String,
    status: Status,
}
impl Todo {
    fn new<S: ToString>(key: TodoKey, label: S) -> Self {
        Self {
            id: key,
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
            Status::Done => Status::Pending,
        };
    }
}

#[component]
pub fn App() -> impl IntoView {
    let store = Store::new(Data::default());

    let input_ref = NodeRef::new();

    view! {
        <form on:submit=move |ev| {
            ev.prevent_default();
            store
                .todos()
                .write()
                .insert_with_key(|key| { Todo::new(key, input_ref.get().unwrap().value()) });
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
fn TodoRow(
    store: Store<Data>,
    #[prop(into)] todo: Field<Todo>,
) -> impl IntoView {
    let status = todo.status();
    let title = todo.label();

    let editing = RwSignal::new(false);

    view! {
        <li style:text-decoration=move || {
            if todo.status().done() { "line-through" } else { Default::default() }
        }>
            <span
                class:hidden=move || editing.get()
                on:click=move |_| {
                    editing.update(|n| *n = !*n);
                }
            >
                {move || title.get()}
            </span>

            <input
                class:hidden=move || !(editing.get())
                type="text"
                prop:value=move || title.get()
                on:change=move |ev| {
                    title.set(event_target_value(&ev));
                }
                on:keyup=move |e| {
                    if e.key_code() == 13 {
                        editing.set(false);
                    }
                }
                on:focusout=move |_| editing.set(false)
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
                store.todos().write().retain(|_, todo| todo.id != id);
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
