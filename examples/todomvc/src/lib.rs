use leptos::{web_sys::HtmlInputElement, *};
use storage::TodoSerialized;
use uuid::Uuid;

mod storage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Todos(pub Vec<Todo>);

const STORAGE_KEY: &str = "todos-leptos";

// Basic operations to manipulate the todo list: nothing really interesting here
impl Todos {
    pub fn new(cx: Scope) -> Self {
        let starting_todos = if is_server!() {
            Vec::new()
        } else if let Ok(Some(storage)) = window().local_storage() {
            storage
                .get_item(STORAGE_KEY)
                .ok()
                .flatten()
                .and_then(|value| serde_json::from_str::<Vec<TodoSerialized>>(&value).ok())
                .map(|values| {
                    values
                        .into_iter()
                        .map(|stored| stored.into_todo(cx))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self(starting_todos)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, todo: Todo) {
        self.0.push(todo);
    }

    pub fn remove(&mut self, id: Uuid) {
        self.0.retain(|todo| todo.id != id);
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
                if todo.completed.get() {
                    todo.completed.set(false);
                }
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
        self.0.retain(|todo| !todo.completed.get());
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Todo {
    pub id: Uuid,
    pub title: RwSignal<String>,
    pub completed: RwSignal<bool>,
}

impl Todo {
    pub fn new(cx: Scope, id: Uuid, title: String) -> Self {
        Self::new_with_completed(cx, id, title, false)
    }

    pub fn new_with_completed(cx: Scope, id: Uuid, title: String, completed: bool) -> Self {
        // RwSignal combines the getter and setter in one struct, rather than separating
        // the getter from the setter. This makes it more convenient in some cases, such
        // as when we're putting the signals into a struct and passing it around. There's
        // no real difference: you could use `create_signal` here, or use `create_rw_signal`
        // everywhere.
        let title = create_rw_signal(cx, title);
        let completed = create_rw_signal(cx, completed);
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
pub fn TodoMVC(cx: Scope) -> Element {
    // The `todos` are a signal, since we need to reactively update the list
    let (todos, set_todos) = create_signal(cx, Todos::new(cx));

    // We provide a context that each <Todo/> component can use to update the list
    // Here, I'm just passing the `WriteSignal`; a <Todo/> doesn't need to read the whole list
    // (and shouldn't try to, as that would cause each individual <Todo/> to re-render when
    // a new todo is added! This kind of hygiene is why `create_signal` defaults to read-write
    // segregation.)
    provide_context(cx, set_todos);

    // Handle the three filter modes: All, Active, and Completed
    let (mode, set_mode) = create_signal(cx, Mode::All);
    window_event_listener("hashchange", move |_| {
        let new_mode = location_hash().map(|hash| route(&hash)).unwrap_or_default();
        set_mode(new_mode);
    });

    // Callback to add a todo on pressing the `Enter` key, if the field isn't empty
    let add_todo = move |ev: web_sys::Event| {
        let target = event_target::<HtmlInputElement>(&ev);
        ev.stop_propagation();
        let key_code = ev.unchecked_ref::<web_sys::KeyboardEvent>().key_code();
        if key_code == ENTER_KEY {
            let title = event_target_value(&ev);
            let title = title.trim();
            if !title.is_empty() {
                let new = Todo::new(cx, Uuid::new_v4(), title.to_string());
                set_todos.update(|t| t.add(new));
                target.set_value("");
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
    // this is the main point of `create_effect`: to synchronize reactive state
    // with something outside the reactive system (like localStorage)
    create_effect(cx, move |_| {
        if let Ok(Some(storage)) = window().local_storage() {
            let objs = todos
                .get()
                .0
                .iter()
                .map(TodoSerialized::from)
                .collect::<Vec<_>>();
            let json = serde_json::to_string(&objs).expect("couldn't serialize Todos");
            if storage.set_item(STORAGE_KEY, &json).is_err() {
                log::error!("error while trying to set item in localStorage");
            }
        }
    });

    view! { cx,
        <main>
            <section class="todoapp">
                <header class="header">
                    <h1>"todos"</h1>
                    <input
                        class="new-todo"
                        placeholder="What needs to be done?"
                        autofocus
                        on:keydown=add_todo
                    />
                </header>
                <section
                    class="main"
                    class:hidden={move || todos.with(|t| t.is_empty())}
                >
                    <input id="toggle-all" class="toggle-all" type="checkbox"
                        prop:checked={move || todos.with(|t| t.remaining() > 0)}
                        on:input=move |_| set_todos.update(|t| t.toggle_all())
                    />
                    <label for="toggle-all">"Mark all as complete"</label>
                    <ul class="todo-list">
                        <For each=filtered_todos key=|todo| todo.id>
                            {move |cx, todo: &Todo| view! { cx,  <Todo todo=todo.clone() /> }}
                        </For>
                    </ul>
                </section>
                <footer
                    class="footer"
                    class:hidden={move || todos.with(|t| t.is_empty())}
                >
                    <span class="todo-count">
                        <strong>{move || todos.with(|t| t.remaining().to_string())}</strong>
                        {move || if todos.with(|t| t.remaining()) == 1 {
                            " item"
                        } else {
                            " items"
                        }}
                        " left"
                    </span>
                    <ul class="filters">
                        <li><a href="#/" class="selected" class:selected={move || mode() == Mode::All}>"All"</a></li>
                        <li><a href="#/active" class:selected={move || mode() == Mode::Active}>"Active"</a></li>
                        <li><a href="#/completed" class:selected={move || mode() == Mode::Completed}>"Completed"</a></li>
                    </ul>
                    <button
                        class="clear-completed hidden"
                        class:hidden={move || todos.with(|t| t.completed() == 0)}
                        on:click=move |_| set_todos.update(|t| t.clear_completed())
                    >
                        "Clear completed"
                    </button>
                </footer>
            </section>
            <footer class="info">
                <p>"Double-click to edit a todo"</p>
                <p>"Created by "<a href="http://todomvc.com">"Greg Johnston"</a></p>
                <p>"Part of "<a href="http://todomvc.com">"TodoMVC"</a></p>
            </footer>
        </main>
    }
}

#[component]
pub fn Todo(cx: Scope, todo: Todo) -> Element {
    let (editing, set_editing) = create_signal(cx, false);
    let set_todos = use_context::<WriteSignal<Todos>>(cx).unwrap();

    // this will be filled by _ref=input below
    let input: Element;

    let save = move |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            set_todos.update(|t| t.remove(todo.id));
        } else {
            todo.title.set(value.to_string());
        }
        set_editing(false);
    };

    // the `input` variable above is filled by a ref, when the template is created
    // so we create the template and store it in a variable here, so we can
    // set up an effect using the `input` below
    let tpl = view! { cx,
        <li
            class="todo"
            class:editing={editing}
            class:completed={move || todo.completed.get()}
            _ref=input
        >
            <div class="view">
                <input
                    class="toggle"
                    type="checkbox"
                    prop:checked={move || (todo.completed)()}
                    on:input={move |ev| {
                        let checked = event_target_checked(&ev);
                        todo.completed.set(checked);
                    }}
                />
                <label on:dblclick=move |_| set_editing(true)>
                    {move || todo.title.get()}
                </label>
                <button class="destroy" on:click=move |_| set_todos.update(|t| t.remove(todo.id))/>
            </div>
            {move || editing().then(|| view! { cx,
                <input
                    class="edit"
                    class:hidden={move || !(editing)()}
                    prop:value={move || todo.title.get()}
                    on:focusout=move |ev| save(&event_target_value(&ev))
                    on:keyup={move |ev| {
                        let key_code = ev.unchecked_ref::<web_sys::KeyboardEvent>().key_code();
                        if key_code == ENTER_KEY {
                            save(&event_target_value(&ev));
                        } else if key_code == ESCAPE_KEY {
                            set_editing(false);
                        }
                    }}
                />
            })
        }
        </li>
    };

    // toggling to edit mode should focus the input
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    create_effect(cx, move |_| {
        if editing() {
            if let Some(input) = input.dyn_ref::<HtmlInputElement>() {
                input.focus();
            }
        }
    });

    tpl
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Active,
    Completed,
    All,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::All
    }
}

pub fn route(hash: &str) -> Mode {
    match hash {
        "/active" => Mode::Active,
        "/completed" => Mode::Completed,
        _ => Mode::All,
    }
}
