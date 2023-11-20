# `<ActionForm/>`

[`<ActionForm/>`](https://docs.rs/leptos_router/latest/leptos_router/fn.ActionForm.html) is a specialized `<Form/>` that takes a server action, and automatically dispatches it on form submission. This allows you to call a server function directly from a `<form>`, even without JS/WASM.

The process is simple:

1. Define a server function using the [`#[server]` macro](https://docs.rs/leptos/latest/leptos/attr.server.html) (see [Server Functions](../server/25_server_functions.md).)
2. Create an action using [`create_server_action`](https://docs.rs/leptos/latest/leptos/fn.create_server_action.html), specifying the type of the server function you’ve defined.
3. Create an `<ActionForm/>`, providing the server action in the `action` prop.
4. Pass the named arguments to the server function as form fields with the same names.

> **Note:** `<ActionForm/>` only works with the default URL-encoded `POST` encoding for server functions, to ensure graceful degradation/correct behavior as an HTML form.

```rust
#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    todo!()
}

#[component]
fn AddTodo() -> impl IntoView {
	let add_todo = create_server_action::<AddTodo>();
	// holds the latest *returned* value from the server
	let value = add_todo.value();
	// check if the server has returned an error
	let has_error = move || value.with(|val| matches!(val, Some(Err(_))));

	view! {
		<ActionForm action=add_todo>
			<label>
				"Add a Todo"
				// `title` matches the `title` argument to `add_todo`
				<input type="text" name="title"/>
			</label>
			<input type="submit" value="Add"/>
		</ActionForm>
	}
}
```

It’s really that easy. With JS/WASM, your form will submit without a page reload, storing its most recent submission in the `.input()` signal of the action, its pending status in `.pending()`, and so on. (See the [`Action`](https://docs.rs/leptos/latest/leptos/struct.Action.html) docs for a refresher, if you need.) Without JS/WASM, your form will submit with a page reload. If you call a `redirect` function (from `leptos_axum` or `leptos_actix`) it will redirect to the correct page. By default, it will redirect back to the page you’re currently on. The power of HTML, HTTP, and isomorphic rendering mean that your `<ActionForm/>` simply works, even with no JS/WASM.

## Client-Side Validation

Because the `<ActionForm/>` is just a `<form>`, it fires a `submit` event. You can use either HTML validation, or your own client-side validation logic in an `on:submit`. Just call `ev.prevent_default()` to prevent submission.

The [`FromFormData`](https://docs.rs/leptos_router/latest/leptos_router/trait.FromFormData.html) trait can be helpful here, for attempting to parse your server function’s data type from the submitted form.

```rust
let on_submit = move |ev| {
	let data = AddTodo::from_event(&ev);
	// silly example of validation: if the todo is "nope!", nope it
	if data.is_err() || data.unwrap().title == "nope!" {
		// ev.prevent_default() will prevent form submission
		ev.prevent_default();
	}
}
```

## Complex Inputs

Server function arguments that are structs with nested serializable fields should make use of indexing notation of `serde_qs`.

```rust
use leptos::*;
use leptos_router::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct HeftyData {
    first_name: String,
    last_name: String,
}

#[component]
fn ComplexInput() -> impl IntoView {
    let submit = Action::<VeryImportantFn, _>::server();

    view! {
      <ActionForm action=submit>
        <input type="text" name="hefty_arg[first_name]" value="leptos"/>
        <input
          type="text"
          name="hefty_arg[last_name]"
          value="closures-everywhere"
        />
        <input type="submit"/>
      </ActionForm>
    }
}

#[server]
async fn very_important_fn(
    hefty_arg: HeftyData,
) -> Result<(), ServerFnError> {
    assert_eq!(hefty_arg.first_name.as_str(), "leptos");
    assert_eq!(hefty_arg.last_name.as_str(), "closures-everywhere");
    Ok(())
}

```
