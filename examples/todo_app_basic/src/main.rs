use leptos::*;

fn main() {
    mount_to_body(|cx| view! { cx, <App /> });
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct TodoItem {
    id: u32,
    name: String,
}

#[derive(Copy, Clone)]
struct TodoGenerator {
    next_id: u32,
}

impl TodoGenerator {
    fn new() -> Self {
        TodoGenerator { next_id: 1 }
    }
    fn get_todo(&mut self, name: String) -> TodoItem {
        let next_id = self.next_id;
        self.next_id += 1;
        TodoItem { name, id: next_id }
    }
}

#[component]
fn App(cx: Scope) -> impl IntoView {
    let (draft_item_name, set_draft_item_name) =
        create_signal(cx, String::new());
    let (todos, set_todos) = create_signal::<Vec<TodoItem>>(cx, vec![]);
    let mut todo_generator = TodoGenerator::new();

    let delete_todo = move |id: u32| {
        set_todos.update(move |todos| {
            todos.retain(|t| t.id != id);
        });
    };

    view! {
        cx,
        <form on:submit=move |e: ev::SubmitEvent| {
            e.prevent_default();
            set_todos.update(|todos| {
                todos.push(todo_generator.get_todo(draft_item_name()));
                set_draft_item_name(String::new())
            });
        }>
            <label for="todo-name">"Todo Name: "</label>
            <input
                id="todo-name"
                type="text"
                prop:value=draft_item_name
                on:input=move |e| {
                    let value = event_target_value(&e);
                    set_draft_item_name(value);
                }
            />
        </form>
        <For
            each=todos
            key=move |todo| todo.id
            view=move |cx, todo| view! { cx, <TodoItem todo=todo delete_todo=Box::new(delete_todo) /> }
        />
    }
}

#[component]
fn TodoItem(
    cx: Scope,
    todo: TodoItem,
    delete_todo: Box<dyn Fn(u32)>,
) -> impl IntoView {
    // Note that we could have structured our todo list as a vector of signals.
    // That would be a bit noisier, but would allow us to implement inline
    // editing of TODO items inside this component, where we'd recieve a signal
    // here instead of a TODO item, such that we could write to that signal
    // to propagate changes throughout our app!
    view! {
        cx,
        <p>{todo.name.clone()}</p>
        <button on:click=move |_| delete_todo(todo.id)>"Done"</button>
    }
}
