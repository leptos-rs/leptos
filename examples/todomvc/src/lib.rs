use leptos::{web_sys::HtmlInputElement, *};
use miniserde::json;
use storage::TodoSerialized;

mod storage;

#[derive(Clone)]
pub struct Todos {
    todos: ReadSignal<Vec<Todo>>,
    set_todos: WriteSignal<Vec<Todo>>,
}

const STORAGE_KEY: &str = "todos-leptos";

impl Todos {
    pub fn new(cx: Scope) -> Self {
        let starting_todos = if let Ok(Some(storage)) = window().local_storage() {
            storage
                .get_item(STORAGE_KEY)
                .ok()
                .flatten()
                .and_then(|value| json::from_str::<Vec<TodoSerialized>>(&value).ok())
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
        let (todos, set_todos) = cx.signal_cloned(starting_todos);
        Self { todos, set_todos }
    }

    pub fn is_empty(&self) -> bool {
        self.todos.get().is_empty()
    }

    pub fn add(&self, todo: Todo) {
        (self.set_todos)(move |todos| todos.push(todo.clone()));
    }

    pub fn remove(&self, id: usize) {
        (self.set_todos)(|todos| todos.retain(|todo| todo.id != id));
    }

    pub fn remaining(&self) -> usize {
        self.todos
            .get()
            .iter()
            .filter(|todo| !*todo.completed.get())
            .count()
    }

    pub fn completed(&self) -> usize {
        self.todos
            .get()
            .iter()
            .filter(|todo| *todo.completed.get())
            .count()
    }

    pub fn toggle_all(&self) {
        // if all are complete, mark them all active instead
        if self.remaining() == 0 {
            for todo in self.todos.get_untracked().iter() {
                if todo.is_completed_untracked() {
                    (todo.set_completed)(|completed| *completed = false);
                }
            }
        }
        // otherwise, mark them all complete
        else {
            for todo in self.todos.get_untracked().iter() {
                (todo.set_completed)(|completed| *completed = true);
            }
        }
    }

    fn clear_completed(&self) {
        (self.set_todos)(|todos| todos.retain(|todo| !todo.is_completed_untracked()));
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Todo {
    pub id: usize,
    pub title: ReadSignal<String>,
    pub set_title: WriteSignal<String>,
    pub completed: ReadSignal<bool>,
    pub set_completed: WriteSignal<bool>,
}

impl Todo {
    pub fn new(cx: Scope, id: usize, title: String) -> Self {
        Self::new_with_completed(cx, id, title, false)
    }

    pub fn new_with_completed(cx: Scope, id: usize, title: String, completed: bool) -> Self {
        let (title, set_title) = cx.signal_cloned(title);
        let (completed, set_completed) = cx.signal_cloned(completed);
        Self {
            id,
            title,
            set_title,
            completed,
            set_completed,
        }
    }

    pub fn toggle(&self) {
        (self.set_completed)(|completed| *completed = !*completed);
    }

    pub fn is_completed_untracked(&self) -> bool {
        *self.completed.get_untracked()
    }
}

const ESCAPE_KEY: u32 = 27;
const ENTER_KEY: u32 = 13;

#[component]
pub fn TodoMVC(cx: Scope) -> Vec<Element> {
    let todos = Todos::new(cx);
    cx.provide_context(todos.clone());
    let todos = cx.create_ref(todos);

    let (mode, set_mode) = cx.signal(Mode::All);
    window_event_listener("hashchange", move |_| {
        let new_mode = location_hash().map(|hash| route(&hash)).unwrap_or_default();
        set_mode(|mode| *mode = new_mode);
    });

    let mut next_id = 0;
    let add_todo = move |ev: web_sys::Event| {
        let target = event_target::<HtmlInputElement>(&ev);
        ev.stop_propagation();
        let key_code = ev.unchecked_ref::<web_sys::KeyboardEvent>().key_code();
        if key_code == ENTER_KEY {
            let title = event_target_value(&ev);
            let title = title.trim();
            if !title.is_empty() {
                todos.add(Todo::new(cx, next_id, title.to_string()));
                next_id += 1;
                target.set_value("");
            }
        }
    };

    let filtered_todos = cx.memo::<Vec<Todo>>(move || {
        let todos = todos.todos.get();
        match *mode.get() {
            Mode::All => todos.iter().cloned().collect(),
            Mode::Active => todos
                .iter()
                .filter(|todo| !*todo.completed.get())
                .cloned()
                .collect(),
            Mode::Completed => todos
                .iter()
                .filter(|todo| *todo.completed.get())
                .cloned()
                .collect(),
        }
    });

    // effect to serialize to JSON
    // this does reactive reads, so it will automatically serialize on any relevant change
    cx.create_effect(move || {
        if let Ok(Some(storage)) = window().local_storage() {
            let objs = todos
                .todos
                .get()
                .iter()
                .map(TodoSerialized::from)
                .collect::<Vec<_>>();
            let json = json::to_string(&objs);
            storage.set_item(STORAGE_KEY, &json);
        }
    });

    view! {
        <>
            <section class="todoapp">
                <header class="header">
                    <h1>"todos"</h1>
                    <input class="new-todo" placeholder="What needs to be done?" autofocus on:keydown={add_todo} />
                </header>
                <section class="main" class:hidden={move || todos.is_empty()}>
                    <input id="toggle-all" class="toggle-all" type="checkbox"
                        prop:checked={move || todos.remaining() > 0}
                        on:input={move |_| todos.toggle_all()}
                    />
                    <label for="toggle-all">"Mark all as complete"</label>
                    <ul class="todo-list">
                        <For each={filtered_todos} key={|todo| todo.id}>
                            {move |cx, todo| view! { <Todo todo={todo.clone()} /> }}
                        </For>
                    </ul>
                </section>
                <footer class="footer" class:hidden={move || todos.is_empty()}>
                    <span class="todo-count">
                        <strong>{move || todos.remaining().to_string()}</strong>
                        {move || if todos.remaining() == 1 {
                            " item"
                        } else {
                            " items"
                        }}
                        " left"
                    </span>
                    <ul class="filters">
                        <li><a href="#/" class:selected={move || *mode.get() == Mode::All}>"All"</a></li>
                        <li><a href="#/active" class:selected={move || *mode.get() == Mode::Active}>"Active"</a></li>
                        <li><a href="#/completed" class:selected={move || *mode.get() == Mode::Completed}>"Completed"</a></li>
                    </ul>
                    <button
                        class="clear-completed"
                        class:hidden={move || todos.completed() == 0}
                        on:click={move |_| todos.clear_completed()}
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
        </>
    }
}

#[component]
pub fn Todo(cx: Scope, todo: Todo) -> Element {
    // creates a scope-bound reference to the Todo
    // this allows us to move the reference into closures below without cloning it
    let todo = cx.create_ref(todo);
    let (editing, set_editing) = cx.signal(false);
    let todos = cx.use_context::<Todos>().unwrap();
    let input: web_sys::Element;

    let save = move |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            todos.remove(todo.id);
        } else {
            (todo.set_title)(move |n| *n = value.to_string());
        }
        set_editing(|n| *n = false);
    };

    let tpl = view! {
        <li
            class="todo"
            class:editing={move || *editing.get()}
            class:completed={move || *todo.completed.get()}
            _ref=input
        >
            <div class="view">
                <input
                    class="toggle"
                    type="checkbox"
                    prop:checked={move || *todo.completed.get()}
                    on:input={move |ev| {
                        let checked = event_target_checked(&ev);
                        (todo.set_completed)(|n| *n = checked);
                    }}
                />
                <label on:dblclick={move |_| set_editing(|n| *n = true)}>
                    {move || todo.title.get().clone()}
                </label>
                <button class="destroy" on:click={move |_| todos.remove(todo.id)}/>
            </div>
            {move || (*editing.get()).then(|| view! {
                <input
                    class="edit"
                    class:hidden={move || !*editing.get()}
                    prop:value={move || todo.title.get().to_string()}
                    on:focusout={move |ev| save(&event_target_value(&ev))}
                    on:keyup={move |ev| {
                        let key_code = ev.unchecked_ref::<web_sys::KeyboardEvent>().key_code();
                        if key_code == ENTER_KEY {
                            save(&event_target_value(&ev));
                        } else if key_code == ESCAPE_KEY {
                            set_editing(|n| *n = false);
                        }
                    }}
                />
            })
        }
        </li>
    };

    cx.create_effect(move || {
        if *editing.get() {
            log!("focusing element");
            input.unchecked_ref::<HtmlInputElement>().focus();
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
