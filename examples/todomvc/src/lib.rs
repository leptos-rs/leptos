use leptos::ev;
use leptos::html::Input;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use web_sys::KeyboardEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todos(pub Vec<Todo>);

const STORAGE_KEY: &str = "todos-leptos";

impl Default for Todos {
    fn default() -> Self {
        let starting_todos =
            window()
                .local_storage()
                .ok()
                .flatten()
                .and_then(|storage| {
                    storage.get_item(STORAGE_KEY).ok().flatten().and_then(
                        |value| serde_json::from_str::<Vec<Todo>>(&value).ok(),
                    )
                })
                .unwrap_or_default();
        Self(starting_todos)
    }
}

// Basic operations to manipulate the todo list: nothing really interesting here
impl Todos {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, todo: Todo) {
        self.0.push(todo);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.retain(|todo| todo.id != id);
    }

    pub fn remaining(&self) -> usize {
        // `todo.completed` is a signal, so we call .get() to access its value
        self.0.iter().filter(|todo| !todo.completed.get()).count()
    }

    pub fn completed(&self) -> usize {
        // `todo.completed` is a signal, so we call .get() to access its value
        self.0.iter().filter(|todo| todo.completed.get()).count()
    }

    pub fn toggle_all(&self) {
        // if all are complete, mark them all active
        if self.remaining() == 0 {
            for todo in &self.0 {
                todo.completed.update(|completed| {
                    if *completed {
                        *completed = false
                    }
                });
            }
        }
        // otherwise, mark them all complete
        else {
            for todo in &self.0 {
                todo.completed.set(true);
            }
        }
    }

    fn clear_completed(&mut self) {
        self.retain(|todo| !todo.completed.get());
    }

    fn retain(&mut self, mut f: impl FnMut(&Todo) -> bool) {
        self.0.retain(|todo| {
            let retain = f(todo);
            // because these signals are created at the top level,
            // they are owned by the <TodoMVC/> component and not
            // by the individual <Todo/> components. This means
            // that if they are not manually disposed when removed, they
            // will be held onto until the <TodoMVC/> is unmounted.
            if !retain {
                todo.title.dispose();
                todo.completed.dispose();
            }
            retain
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
}

impl Todo {
    pub fn new(id: Uuid, title: String) -> Self {
        Self::new_with_completed(id, title, false)
    }

    pub fn new_with_completed(
        id: Uuid,
        title: String,
        completed: bool,
    ) -> Self {
        // RwSignal combines the getter and setter in one struct, rather than separating
        // the getter from the setter. This makes it more convenient in some cases, such
        // as when we're putting the signals into a struct and passing it around.
        let title = RwSignal::new(title);
        let completed = RwSignal::new(completed);
        Self {
            id,
            title,
            completed,
        }
    }

    pub fn toggle(&self) {
        // A signal's `update()` function gives you a mutable reference to the current value
        // You can use that to modify the value in place, which will notify any subscribers.
        self.completed.update(|completed| *completed = !*completed);
    }
}

const ESCAPE_KEY: u32 = 27;
const ENTER_KEY: u32 = 13;

#[component]
pub fn TodoMVC() -> impl IntoView {
    // The `todos` are a signal, since we need to reactively update the list
    let (todos, set_todos) = signal(Todos::default());

    // We provide a context that each <Todo/> component can use to update the list
    // Here, I'm just passing the `WriteSignal`; a <Todo/> doesn't need to read the whole list
    // (and shouldn't try to, as that would cause each individual <Todo/> to re-render when
    // a new todo is added! This kind of hygiene is why `signal` defaults to read-write
    // segregation.)
    provide_context(set_todos);

    // Handle the three filter modes: All, Active, and Completed
    let (mode, set_mode) = signal(Mode::All);

    window_event_listener(ev::hashchange, move |_| {
        let new_mode =
            location_hash().map(|hash| route(&hash)).unwrap_or_default();
        set_mode.set(new_mode);
    });

    // Callback to add a todo on pressing the `Enter` key, if the field isn't empty
    let input_ref = NodeRef::<Input>::new();
    let add_todo = move |ev: KeyboardEvent| {
        let input = input_ref.get().unwrap();
        ev.stop_propagation();
        let key_code = ev.key_code();
        if key_code == ENTER_KEY {
            let title = input.value();
            let title = title.trim();
            if !title.is_empty() {
                let new = Todo::new(Uuid::new_v4(), title.to_string());
                set_todos.update(|t| t.add(new));
                input.set_value("");
            }
        }
    };

    // A derived signal that filters the list of the todos depending on the filter mode
    // This doesn't need to be a `Memo`, because we're only reading it in one place
    let filtered_todos = move || {
        todos.with(|todos| match mode.get() {
            Mode::All => todos.0.to_vec(),
            Mode::Active => todos
                .0
                .iter()
                .filter(|todo| !todo.completed.get())
                .cloned()
                .collect(),
            Mode::Completed => todos
                .0
                .iter()
                .filter(|todo| todo.completed.get())
                .cloned()
                .collect(),
        })
    };

    // Serialization
    //
    // the effect reads the `todos` signal, and each `Todo`'s title and completed
    // status,  so it will automatically re-run on any change to the list of tasks
    //
    // this is the main point of effects: to synchronize reactive state
    // with something outside the reactive system (like localStorage)

    Effect::new(move |_| {
        if let Ok(Some(storage)) = window().local_storage() {
            let json = serde_json::to_string(&todos)
                .expect("couldn't serialize Todos");
            if storage.set_item(STORAGE_KEY, &json).is_err() {
                leptos::logging::error!(
                    "error while trying to set item in localStorage"
                );
            }
        }
    });

    // focus the main input on load
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    view! {
        <main>
            <section class="todoapp">
                <header class="header">
                    <h1>"todos"</h1>
                    <input
                        class="new-todo"
                        placeholder="What needs to be done?"
                        autofocus
                        on:keydown=add_todo
                        node_ref=input_ref
                    />
                </header>
                <section class="main" class:hidden=move || todos.with(|t| t.is_empty())>
                    <input
                        id="toggle-all"
                        class="toggle-all"
                        type="checkbox"
                        prop:checked=move || todos.with(|t| t.remaining() > 0)
                        on:input=move |_| todos.with(|t| t.toggle_all())
                    />
                    <label for="toggle-all">"Mark all as complete"</label>
                    <ul class="todo-list">
                        <For each=filtered_todos key=|todo| todo.id let:todo>
                            <Todo todo/>
                        </For>
                    </ul>
                </section>
                <footer class="footer" class:hidden=move || todos.with(|t| t.is_empty())>
                    <span class="todo-count">
                        <strong>{move || todos.with(|t| t.remaining().to_string())}</strong>
                        {move || {
                            if todos.with(|t| t.remaining()) == 1 { " item" } else { " items" }
                        }}

                        " left"
                    </span>
                    <ul class="filters">
                        <li>
                            <a
                                href="#/"
                                class="selected"
                                class:selected=move || mode.get() == Mode::All
                            >
                                "All"
                            </a>
                        </li>
                        <li>
                            <a href="#/active" class:selected=move || mode.get() == Mode::Active>
                                "Active"
                            </a>
                        </li>
                        <li>
                            <a
                                href="#/completed"
                                class:selected=move || mode.get() == Mode::Completed
                            >
                                "Completed"
                            </a>
                        </li>
                    </ul>
                    <button
                        class="clear-completed hidden"
                        class:hidden=move || todos.with(|t| t.completed() == 0)
                        on:click=move |_| set_todos.update(|t| t.clear_completed())
                    >
                        "Clear completed"
                    </button>
                </footer>
            </section>
            <footer class="info">
                <p>"Double-click to edit a todo"</p>
                <p>"Created by " <a href="http://todomvc.com">"Greg Johnston"</a></p>
                <p>"Part of " <a href="http://todomvc.com">"TodoMVC"</a></p>
            </footer>
        </main>
    }
}

#[component]
pub fn Todo(todo: Todo) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let set_todos = use_context::<WriteSignal<Todos>>().unwrap();

    // this will be filled by node_ref=input below
    let todo_input = NodeRef::<Input>::new();

    let save = move |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            set_todos.update(|t| t.remove(todo.id));
        } else {
            todo.title.set(value.to_string());
        }
        set_editing.set(false);
    };

    view! {
        <li class="todo" class:editing=editing class:completed=move || todo.completed.get()>
            <div class="view">
                <input
                    node_ref=todo_input
                    class="toggle"
                    type="checkbox"
                    bind:checked=todo.completed
                />

                <label on:dblclick=move |_| {
                    set_editing.set(true);
                    if let Some(input) = todo_input.get() {
                        _ = input.focus();
                    }
                }>{move || todo.title.get()}</label>
                <button
                    class="destroy"
                    on:click=move |_| set_todos.update(|t| t.remove(todo.id))
                ></button>
            </div>
            {move || {
                editing
                    .get()
                    .then(|| {
                        view! {
                            <input
                                class="edit"
                                class:hidden=move || !editing.get()
                                prop:value=move || todo.title.get()
                                on:focusout:target=move |ev| save(&ev.target().value())
                                on:keyup:target=move |ev| {
                                    let key_code = ev.key_code();
                                    if key_code == ENTER_KEY {
                                        save(&ev.target().value());
                                    } else if key_code == ESCAPE_KEY {
                                        set_editing.set(false);
                                    }
                                }
                            />
                        }
                    })
            }}

        </li>
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Active,
    Completed,
    #[default]
    All,
}

pub fn route(hash: &str) -> Mode {
    match hash {
        "/active" => Mode::Active,
        "/completed" => Mode::Completed,
        _ => Mode::All,
    }
}
