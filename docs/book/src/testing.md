# Testing Your Components

Testing user interfaces can be relatively tricky, but really important. This article
will discuss a couple principles and approaches for testing a Leptos app.

## 1. Test business logic with ordinary Rust tests

In many cases, it makes sense to pull the logic out of your components and test
it separately. For some simple components, there’s no particular logic to test, but
for many it’s worth using a testable wrapping type and implementing the logic in
ordinary Rust `impl` blocks.

For example, instead of embedding logic in a component directly like this:

```rust
#[component]
pub fn TodoApp() -> impl IntoView {
    let (todos, set_todos) = create_signal(vec![Todo { /* ... */ }]);
    // ⚠️ this is hard to test because it's embedded in the component
    let num_remaining = move || todos.with(|todos| {
        todos.iter().filter(|todo| !todo.completed).sum()
    });
}
```

You could pull that logic out into a separate data structure and test it:

```rust
pub struct Todos(Vec<Todo>);

impl Todos {
    pub fn num_remaining(&self) -> usize {
        todos.iter().filter(|todo| !todo.completed).sum()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_remaining {
        // ...
    }
}

#[component]
pub fn TodoApp() -> impl IntoView {
    let (todos, set_todos) = create_signal(Todos(vec![Todo { /* ... */ }]));
    // ✅ this has a test associated with it
    let num_remaining = move || todos.with(Todos::num_remaining);
}
```

In general, the less of your logic is wrapped into your components themselves, the
more idiomatic your code will feel and the easier it will be to test.

## 2. Test components with `wasm-bindgen-test`

[`wasm-bindgen-test`](https://crates.io/crates/wasm-bindgen-test) is a great utility
for integrating or end-to-end testing WebAssembly apps in a headless browser.

To use this testing utility, you need to add `wasm-bindgen-test` to your `Cargo.toml`:

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3.0"
```

You should create tests in a separate `tests` directory. You can then run your tests in the browser of your choice:

```bash
wasm-pack test --firefox
```

> To see the full setup, check out the tests for the [`counter`](https://github.com/leptos-rs/leptos/tree/main/examples/counter) example.

### Writing Your Tests

Most tests will involve some combination of vanilla DOM manipulation and comparison to a `view`. For example, here’s a test [for the
`counter` example](https://github.com/leptos-rs/leptos/blob/main/examples/counter/tests/web.rs).

First, we set up the testing environment.

```rust
use wasm_bindgen_test::*;
use counter::*;
use leptos::*;
use web_sys::HtmlElement;

// tell the test runner to run tests in the browser
wasm_bindgen_test_configure!(run_in_browser);
```

I’m going to create a simpler wrapper for each test case, and mount it there.
This makes it easy to encapsulate the test results.

```rust
// like marking a regular test with #[test]
#[wasm_bindgen_test]
fn clear() {
    let document = leptos::document();
    let test_wrapper = document.create_element("section").unwrap();
    document.body().unwrap().append_child(&test_wrapper);

    // start by rendering our counter and mounting it to the DOM
    // note that we start at the initial value of 10
    mount_to(
        test_wrapper.clone().unchecked_into(),
        || view! { <SimpleCounter initial_value=10 step=1/> },
    );
}
```

We’ll use some manual DOM operations to grab the `<div>` that wraps
the whole component, as well as the `clear` button.

```rust
// now we extract the buttons by iterating over the DOM
// this would be easier if they had IDs
let div = test_wrapper.query_selector("div").unwrap().unwrap();
let clear = test_wrapper
    .query_selector("button")
    .unwrap()
    .unwrap()
    .unchecked_into::<web_sys::HtmlElement>();
```

Now we can use ordinary DOM APIs to simulate user interaction.

```rust
// now let's click the `clear` button
clear.click();
```

You can test individual DOM element attributes or text node values. Sometimes
I like to test the whole view at once. We can do this by testing the element’s
`outerHTML` against our expectations.

```rust
assert_eq!(
    div.outer_html(),
    // here we spawn a mini reactive system to render the test case
    run_scope(create_runtime(), || {
        // it's as if we're creating it with a value of 0, right?
        let (value, set_value) = create_signal(0);

        // we can remove the event listeners because they're not rendered to HTML
        view! {
            <div>
                <button>"Clear"</button>
                <button>"-1"</button>
                <span>"Value: " {value} "!"</span>
                <button>"+1"</button>
            </div>
        }
        // the view returned an HtmlElement<Div>, which is a smart pointer for
        // a DOM element. So we can still just call .outer_html()
        .outer_html()
    })
);
```

That test involved us manually replicating the `view` that’s inside the component.
There's actually an easier way to do this... We can just test against a `<SimpleCounter/>`
with the initial value `0`. This is where our wrapping element comes in: I’ll just test
the wrapper’s `innerHTML` against another comparison case.

```rust
assert_eq!(test_wrapper.inner_html(), {
    let comparison_wrapper = document.create_element("section").unwrap();
    leptos::mount_to(
        comparison_wrapper.clone().unchecked_into(),
        || view! { <SimpleCounter initial_value=0 step=1/>},
    );
    comparison_wrapper.inner_html()
});
```

This is only a very limited introduction to testing. But I hope it’s useful as you begin to build applications.

> For more, see [the testing section of the `wasm-bindgen` guide](https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/index.html#testing-on-wasm32-unknown-unknown-with-wasm-bindgen-test).
