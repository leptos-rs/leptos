use serde::{Deserialize, Serialize};
use sycamore::prelude::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement, KeyboardEvent};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Todo {
    title: String,
    completed: bool,
    id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Active,
    Completed,
}

impl Default for Filter {
    fn default() -> Self {
        Self::All
    }
}

impl Filter {
    fn url(self) -> &'static str {
        match self {
            Filter::All => "#",
            Filter::Active => "#/active",
            Filter::Completed => "#/completed",
        }
    }

    fn get_filter_from_hash() -> Self {
        let hash = web_sys::window().unwrap().location().hash().unwrap();

        match hash.as_str() {
            "#/active" => Filter::Active,
            "#/completed" => Filter::Completed,
            _ => Filter::All,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub todos: RcSignal<Vec<RcSignal<Todo>>>,
    pub filter: RcSignal<Filter>,
}

impl AppState {
    fn add_todo(&self, title: String, id: usize) {
        self.todos.modify().push(create_rc_signal(Todo {
            title,
            completed: false,
            id,
        }))
    }

    fn remove_todo(&self, id: usize) {
        self.todos.modify().retain(|todo| todo.get().id != id);
    }

    fn todos_left(&self) -> usize {
        self.todos.get().iter().fold(
            0,
            |acc, todo| if todo.get().completed { acc } else { acc + 1 },
        )
    }

    fn toggle_complete_all(&self) {
        if self.todos_left() == 0 {
            // make all todos active
            for todo in self.todos.get().iter() {
                if todo.get().completed {
                    todo.set(Todo {
                        completed: false,
                        ..todo.get().as_ref().clone()
                    })
                }
            }
        } else {
            // make all todos completed
            for todo in self.todos.get().iter() {
                if !todo.get().completed {
                    todo.set(Todo {
                        completed: true,
                        ..todo.get().as_ref().clone()
                    })
                }
            }
        }
    }

    fn clear_completed(&self) {
        self.todos.modify().retain(|todo| !todo.get().completed);
    }
}

const KEY: &str = "todos-sycamore";

#[component]
pub fn App<G: Html>(cx: Scope) -> View<G> {
    // Initialize application state
    let todos = create_rc_signal(Vec::new());
    let app_state = AppState {
        todos,
        filter: create_rc_signal(Filter::All),
    };
    provide_context(cx, app_state);

    view! { cx,
        div(class="todomvc-wrapper") {
            section(class="todoapp") {
                Header {}
                List {}
                Footer {}
            }
            Copyright {}
        }
    }
}

#[component]
pub fn AppWith1000<G: Html>(cx: Scope) -> View<G> {
    // Initialize application state
    let todos = (0..1000)
        .map(|id| {
            create_rc_signal(Todo {
                title: format!("Todo #{id}"),
                completed: false,
                id,
            })
        })
        .collect();
    let todos = create_rc_signal(todos);
    let app_state = AppState {
        todos,
        filter: create_rc_signal(Filter::All),
    };
    provide_context(cx, app_state);

    view! { cx,
        div(class="todomvc-wrapper") {
            section(class="todoapp") {
                Header {}
                List {}
                Footer {}
            }
            Copyright {}
        }
    }
}

#[component]
pub fn Copyright<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        footer(class="info") {
            p { "Double click to edit a todo" }
            p {
                "Created by "
                a(href="https://github.com/lukechu10", target="_blank") { "lukechu10" }
            }
            p {
                "Part of "
                a(href="http://todomvc.com") { "TodoMVC" }
            }
        }
    }
}

#[component]
pub fn Header<G: Html>(cx: Scope) -> View<G> {
    let app_state = use_context::<AppState>(cx);
    let value = create_signal(cx, String::new());
    let input_ref = create_node_ref(cx);

    let handle_submit = |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();

        if event.key() == "Enter" {
            let mut task = value.get().as_ref().clone();
            task = task.trim().to_string();

            if !task.is_empty() {
                app_state.add_todo(task, 0);
                value.set("".to_string());
                input_ref
                    .get::<DomNode>()
                    .unchecked_into::<HtmlInputElement>()
                    .set_value("");
            }
        }
    };

    view! { cx,
        header(class="header") {
            h1 { "todos" }
            input(ref=input_ref,
                class="new-todo",
                placeholder="What needs to be done?",
                bind:value=value,
                on:keyup=handle_submit,
            )
        }
    }
}

#[component(inline_props)]
pub fn Item<G: Html>(cx: Scope, todo: RcSignal<Todo>) -> View<G> {
    let app_state = use_context::<AppState>(cx);
    // Make `todo` live as long as the scope.
    let todo = create_ref(cx, todo);

    let title = || todo.get().title.clone();
    let completed = create_selector(cx, || todo.get().completed);
    let id = todo.get().id;

    let editing = create_signal(cx, false);
    let input_ref = create_node_ref(cx);
    let value = create_signal(cx, "".to_string());

    let handle_input = |event: Event| {
        let target: HtmlInputElement = event.target().unwrap().unchecked_into();
        value.set(target.value());
    };

    let toggle_completed = |_| {
        todo.set(Todo {
            completed: !todo.get().completed,
            ..todo.get().as_ref().clone()
        });
    };

    let handle_dblclick = move |_| {
        editing.set(true);
        input_ref
            .get::<DomNode>()
            .unchecked_into::<HtmlInputElement>()
            .focus()
            .unwrap();
        value.set(title());
    };

    let handle_blur = move || {
        editing.set(false);

        let mut value = value.get().as_ref().clone();
        value = value.trim().to_string();

        if value.is_empty() {
            app_state.remove_todo(id);
        } else {
            todo.set(Todo {
                title: value,
                ..todo.get().as_ref().clone()
            })
        }
    };

    let handle_submit = move |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();
        match event.key().as_str() {
            "Enter" => handle_blur(),
            "Escape" => {
                input_ref
                    .get::<DomNode>()
                    .unchecked_into::<HtmlInputElement>()
                    .set_value(&title());
                editing.set(false);
            }
            _ => {}
        }
    };

    let handle_destroy = move |_| {
        app_state.remove_todo(id);
    };

    // We need a separate signal for checked because clicking the checkbox will detach the binding
    // between the attribute and the view.
    let checked = create_signal(cx, false);
    create_effect(cx, || {
        // Calling checked.set will also update the `checked` property on the input element.
        checked.set(*completed.get())
    });

    let class = || {
        format!(
            "{} {}",
            if *completed.get() { "completed" } else { "" },
            if *editing.get() { "editing" } else { "" }
        )
    };

    view! { cx,
        li(class=class()) {
            div(class="view") {
                input(
                    class="toggle",
                    type="checkbox",
                    on:input=toggle_completed,
                    bind:checked=checked
                )
                label(on:dblclick=handle_dblclick) {
                    (title())
                }
                button(class="destroy", on:click=handle_destroy)
            }

            (if *editing.get() {
                view! { cx,
                    input(ref=input_ref,
                        class="edit",
                        prop:value=&todo.get().title,
                        on:blur=move |_| handle_blur(),
                        on:keyup=handle_submit,
                        on:input=handle_input,
                    )
                }
            } else {
                View::empty()
            })
        }
    }
}

#[component]
pub fn List<G: Html>(cx: Scope) -> View<G> {
    let app_state = use_context::<AppState>(cx);
    let todos_left = create_selector(cx, || app_state.todos_left());

    let filtered_todos = create_memo(cx, || {
        app_state
            .todos
            .get()
            .iter()
            .filter(|todo| match *app_state.filter.get() {
                Filter::All => true,
                Filter::Active => !todo.get().completed,
                Filter::Completed => todo.get().completed,
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    // We need a separate signal for checked because clicking the checkbox will detach the binding
    // between the attribute and the view.
    let checked = create_signal(cx, false);
    create_effect(cx, || {
        // Calling checked.set will also update the `checked` property on the input element.
        checked.set(*todos_left.get() == 0)
    });

    view! { cx,
        section(class="main") {
            input(
                id="toggle-all",
                class="toggle-all",
                type="checkbox",
                readonly=true,
                bind:checked=checked,
                on:input=|_| app_state.toggle_complete_all()
            )
            label(for="toggle-all")

            ul(class="todo-list") {
                Keyed(
                    iterable=filtered_todos,
                    view=|cx, todo| view! { cx,
                        Item(todo=todo)
                    },
                    key=|todo| todo.get().id,
                )
            }
        }
    }
}

#[component(inline_props)]
pub fn TodoFilter<G: Html>(cx: Scope, filter: Filter) -> View<G> {
    let app_state = use_context::<AppState>(cx);
    let selected = move || filter == *app_state.filter.get();
    let set_filter = |filter| app_state.filter.set(filter);

    view! { cx,
        li {
            a(
                class=if selected() { "selected" } else { "" },
                href=filter.url(),
                on:click=move |_| set_filter(filter),
            ) {
                (format!("{filter:?}"))
            }
        }
    }
}

#[component]
pub fn Footer<G: Html>(cx: Scope) -> View<G> {
    let app_state = use_context::<AppState>(cx);

    let items_text = || match app_state.todos_left() {
        1 => "item",
        _ => "items",
    };

    let has_completed_todos =
        create_selector(cx, || app_state.todos_left() < app_state.todos.get().len());

    let handle_clear_completed = |_| app_state.clear_completed();

    view! { cx,
        footer(class="footer") {
            span(class="todo-count") {
                strong { (app_state.todos_left()) }
                span { " " (items_text()) " left" }
            }
            ul(class="filters") {
                TodoFilter(filter=Filter::All)
                TodoFilter(filter=Filter::Active)
                TodoFilter(filter=Filter::Completed)
            }

            (if *has_completed_todos.get() {
                view! { cx,
                    button(class="clear-completed", on:click=handle_clear_completed) {
                        "Clear completed"
                    }
                }
            } else {
                view! { cx, }
            })
        }
    }
}
