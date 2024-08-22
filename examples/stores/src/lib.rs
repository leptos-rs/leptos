use std::sync::atomic::{AtomicUsize, Ordering};

use leptos::prelude::*;
use reactive_stores::{Field, Store, StoreFieldIterator};
use reactive_stores_macro::Store;

// ID starts higher than 0 because we have a few starting todos by default
static NEXT_ID: AtomicUsize = AtomicUsize::new(3);

#[derive(Debug, Store)]
struct Todos {
    user: String,
    todos: Vec<Todo>,
}

#[derive(Debug, Store)]
struct Todo {
    id: usize,
    label: String,
    status: Status,
}

#[derive(Debug, Default, Clone, Store)]
enum Status {
    #[default]
    Pending,
    Scheduled,
    Done,
}

impl Status {
    pub fn next_step(&mut self) {
        *self = match self {
            Status::Pending => Status::Scheduled,
            Status::Scheduled => Status::Done,
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
        user: "Bob".to_string(),
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
        <p>"Hello, " {move || store.user().get()}</p>
        <form on:submit=move |ev| {
            ev.prevent_default();
            store.todos().write().push(Todo::new(input_ref.get().unwrap().value()));
        }>
            <label>"Add a Todo" <input type="text" node_ref=input_ref/></label>
            <input type="submit"/>
        </form>
        <ol>
            <For each=move || store.todos().iter() key=|row| row.id().get() let:todo>
                <TodoRow store todo/>
            </For>

        </ol>
        <div style="display: flex"></div>
    }
}

#[component]
fn TodoRow(
    store: Store<Todos>,
    #[prop(into)] todo: Field<Todo>,
) -> impl IntoView {
    let status = todo.status();
    let title = todo.label();

    let editing = RwSignal::new(false);

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
                    editing.set(false);
                }

                on:blur=move |_| editing.set(false)
                autofocus
            />
            <button on:click=move |_| {
                status.write().next_step()
            }>
                {move || {
                    if todo.status().done() {
                        "Done"
                    } else if status.scheduled() {
                        "Scheduled"
                    } else {
                        "Pending"
                    }
                }}

            </button>

            <button on:click=move |_| {
                store
                    .todos()
                    .update(|todos| {
                        todos.remove(todo.id().get());
                    });
            }>"X"</button>
        </li>
    }
}
