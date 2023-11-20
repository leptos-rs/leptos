pub use leptos_reactive::*;
use miniserde::*;
use tachy_maccy::view;
use tachydom::{
    html::{
        attribute::global::{ClassAttribute, GlobalAttributes, OnAttribute},
        element::ElementChild,
    },
    renderer::dom::Dom,
    view::{keyed::keyed, Render, RenderHtml},
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Todos(pub Vec<Todo>);

const STORAGE_KEY: &str = "todos-leptos";

impl Todos {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn new_with_1000() -> Self {
        let todos = (0..1000)
            .map(|id| Todo::new(id, format!("Todo #{id}")))
            .collect();
        Self(todos)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, todo: Todo) {
        self.0.push(todo);
    }

    pub fn remove(&mut self, id: usize) {
        self.0.retain(|todo| todo.id != id);
    }

    pub fn remaining(&self) -> usize {
        self.0.iter().filter(|todo| !(todo.completed)()).count()
    }

    pub fn completed(&self) -> usize {
        self.0.iter().filter(|todo| (todo.completed)()).count()
    }

    pub fn toggle_all(&self) {
        // if all are complete, mark them all active instead
        if self.remaining() == 0 {
            for todo in &self.0 {
                if todo.completed.get() {
                    (todo.set_completed)(false);
                }
            }
        }
        // otherwise, mark them all complete
        else {
            for todo in &self.0 {
                (todo.set_completed)(true);
            }
        }
    }

    fn clear_completed(&mut self) {
        self.0.retain(|todo| !todo.completed.get());
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Todo {
    pub id: usize,
    pub title: ReadSignal<String>,
    pub set_title: WriteSignal<String>,
    pub completed: ReadSignal<bool>,
    pub set_completed: WriteSignal<bool>,
}

impl Todo {
    pub fn new(id: usize, title: String) -> Self {
        Self::new_with_completed(id, title, false)
    }

    pub fn new_with_completed(
        id: usize,
        title: String,
        completed: bool,
    ) -> Self {
        let (title, set_title) = create_signal(title);
        let (completed, set_completed) = create_signal(completed);
        Self {
            id,
            title,
            set_title,
            completed,
            set_completed,
        }
    }

    pub fn toggle(&self) {
        self.set_completed
            .update(|completed| *completed = !*completed);
    }
}

const ESCAPE_KEY: u32 = 27;
const ENTER_KEY: u32 = 13;

pub fn TodoMVC(todos: Todos) -> impl Render<Dom> + RenderHtml<Dom> {
    let mut next_id = todos
        .0
        .iter()
        .map(|todo| todo.id)
        .max()
        .map(|last| last + 1)
        .unwrap_or(0);

    let (todos, set_todos) = create_signal(todos);
    provide_context(set_todos);

    let (mode, set_mode) = create_signal(Mode::All);

    let add_todo = move |ev: web_sys::KeyboardEvent| {
        todo!()
        /* let target = event_target::<HtmlInputElement>(&ev);
        ev.stop_propagation();
        let key_code = ev.unchecked_ref::<web_sys::KeyboardEvent>().key_code();
        if key_code == ENTER_KEY {
            let title = event_target_value(&ev);
            let title = title.trim();
            if !title.is_empty() {
                let new = Todo::new(next_id, title.to_string());
                set_todos.update(|t| t.add(new));
                next_id += 1;
                target.set_value("");
            }
        } */
    };

    let filtered_todos = create_memo::<Vec<Todo>>(move |_| {
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
    });

    // effect to serialize to JSON
    // this does reactive reads, so it will automatically serialize on any relevant change
    create_effect(move |_| {
        ()
        /* if let Ok(Some(storage)) = window().local_storage() {
            let objs = todos
                .get()
                .0
                .iter()
                .map(TodoSerialized::from)
                .collect::<Vec<_>>();
            let json = json::to_string(&objs);
            if storage.set_item(STORAGE_KEY, &json).is_err() {
                log::error!("error while trying to set item in localStorage");
            }
        } */
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
                    />
                </header>
                <section class="main" class:hidden=move || todos.with(|t| t.is_empty())>
                    <input
                        id="toggle-all"
                        class="toggle-all"
                        r#type="checkbox"
                        //prop:checked=move || todos.with(|t| t.remaining() > 0)
                        on:input=move |_| set_todos.update(|t| t.toggle_all())
                    />
                    <label r#for="toggle-all">"Mark all as complete"</label>
                    <ul class="todo-list">
                        {move || {
                            keyed(filtered_todos.get(), |todo| todo.id, Todo)
                        }}
                    </ul>
                </section>
                <footer class="footer" class:hidden=move || todos.with(|t| t.is_empty())>
                    <span class="todo-count">
                        <strong>{move || todos.with(|t| t.remaining().to_string())}</strong>
                        {move || if todos.with(|t| t.remaining()) == 1 { " item" } else { " items" }}
                        " left"
                    </span>
                    <ul class="filters">
                        <li>
                            <a
                                href="#/"
                                class="selected"
                                class:selected=move || mode() == Mode::All
                            >
                                "All"
                            </a>
                        </li>
                        <li>
                            <a href="#/active" class:selected=move || mode() == Mode::Active>
                                "Active"
                            </a>
                        </li>
                        <li>
                            <a href="#/completed" class:selected=move || mode() == Mode::Completed>
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

pub fn Todo(todo: Todo) -> impl Render<Dom> + RenderHtml<Dom> {
    let (editing, set_editing) = create_signal(false);
    let set_todos = use_context::<WriteSignal<Todos>>().unwrap();
    //let input = NodeRef::new();

    let save = move |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            set_todos.update(|t| t.remove(todo.id));
        } else {
            (todo.set_title)(value.to_string());
        }
        set_editing(false);
    };

    view! {
        <li class="todo" class:editing=editing class:completed=move || (todo.completed)()>
            /* <div class="view">
                <input class="toggle" r#type="checkbox"/>
                <label on:dblclick=move |_| set_editing(true)>{move || todo.title.get()}</label>
                <button
                    class="destroy"
                    on:click=move |_| set_todos.update(|t| t.remove(todo.id))
                ></button>
            </div>
            {move || {
                editing()
                    .then(|| {
                        view! {
                            <input
                                class="edit"
                                class:hidden=move || !(editing)()
                            />
                        }
                    })
            }} */
        </li>
    }
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

#[derive(Serialize, Deserialize)]
pub struct TodoSerialized {
    pub id: usize,
    pub title: String,
    pub completed: bool,
}

impl TodoSerialized {
    pub fn into_todo(self) -> Todo {
        Todo::new_with_completed(self.id, self.title, self.completed)
    }
}

impl From<&Todo> for TodoSerialized {
    fn from(todo: &Todo) -> Self {
        Self {
            id: todo.id,
            title: todo.title.get(),
            completed: (todo.completed)(),
        }
    }
}
