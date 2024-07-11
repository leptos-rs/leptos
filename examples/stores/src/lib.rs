use leptos::prelude::*;
use reactive_stores::{
    AtIndex, Store, StoreField, StoreFieldIterator, Subfield,
};
use reactive_stores_macro::Store;

#[derive(Debug, Store)]
struct Todos {
    user: String,
    todos: Vec<Todo>,
}

#[derive(Debug, Store)]
struct Todo {
    label: String,
    completed: bool,
}

impl Todo {
    pub fn new(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            completed: false,
        }
    }
}

fn data() -> Todos {
    Todos {
        user: "Bob".to_string(),
        todos: vec![
            Todo {
                label: "Create reactive store".to_string(),
                completed: true,
            },
            Todo {
                label: "???".to_string(),
                completed: false,
            },
            Todo {
                label: "Profit".to_string(),
                completed: false,
            },
        ],
    }
}

#[component]
pub fn App() -> impl IntoView {
    let store = Store::new(data());

    let input_ref = NodeRef::new();

    let rows = move || {
        store
            .todos()
            .iter()
            .enumerate()
            .map(|(idx, todo)| view! { <TodoRow store idx todo/> })
            .collect_view()
    };

    view! {
        <form on:submit=move |ev| {
            ev.prevent_default();
            store.todos().write().push(Todo::new(input_ref.get().unwrap().value()));
        }>
            <label>"Add a Todo" <input type="text" node_ref=input_ref/></label>
            <input type="submit"/>
        </form>
        <ol>{rows}</ol>
        <div style="display: flex"></div>
    }
}

#[component]
fn TodoRow(
    store: Store<Todos>,
    idx: usize,
    // to be fair, this is gross
    todo: AtIndex<Subfield<Store<Todos>, Todos, Vec<Todo>>, Vec<Todo>>,
) -> impl IntoView {
    let completed = todo.completed();
    let title = todo.label();

    let editing = RwSignal::new(false);

    view! {
        <li
            style:text-decoration=move || {
                completed.get().then_some("line-through").unwrap_or_default()
            }

            class:foo=move || completed.get()
        >
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
            <input
                type="checkbox"
                prop:checked=move || completed.get()
                on:click=move |_| { completed.update(|n| *n = !*n) }
            />

            <button on:click=move |_| {
                store
                    .todos()
                    .update(|todos| {
                        todos.remove(idx);
                    });
            }>"X"</button>
        </li>
    }
}
