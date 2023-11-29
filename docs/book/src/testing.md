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
        self.0.iter().filter(|todo| !todo.completed).sum()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_remaining() {
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

## 2. Test components with end-to-end (`e2e`) testing

Our [`examples`](https://github.com/leptos-rs/leptos/tree/main/examples) directory has several examples with extensive end-to-end testing, using different testing tools.

The easiest way to see how to use these is to take a look at the test examples themselves:

### `wasm-bindgen-test` with [`counter`](https://github.com/leptos-rs/leptos/blob/main/examples/counter/tests/web.rs)

This is a fairly simple manual testing setup that uses the [`wasm-pack test`](https://rustwasm.github.io/wasm-pack/book/commands/test.html) command.

#### Sample Test

````rust
#[wasm_bindgen_test]
fn clear() {
    let document = leptos::document();
    let test_wrapper = document.create_element("section").unwrap();
    let _ = document.body().unwrap().append_child(&test_wrapper);

    mount_to(
        test_wrapper.clone().unchecked_into(),
        || view! { <SimpleCounter initial_value=10 step=1/> },
    );

    let div = test_wrapper.query_selector("div").unwrap().unwrap();
    let clear = test_wrapper
        .query_selector("button")
        .unwrap()
        .unwrap()
        .unchecked_into::<web_sys::HtmlElement>();

    clear.click();

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
}
````

### [`wasm-bindgen-test` with `counters_stable`](https://github.com/leptos-rs/leptos/tree/main/examples/counters_stable/tests/web)

This more developed test suite uses a system of fixtures to refactor the manual DOM manipulation of the `counter` tests and easily test a wide range of cases.

#### Sample Test

```rust
use super::*;
use crate::counters_page as ui;
use pretty_assertions::assert_eq;

#[wasm_bindgen_test]
fn should_increase_the_total_count() {
    // Given
    ui::view_counters();
    ui::add_counter();

    // When
    ui::increment_counter(1);
    ui::increment_counter(1);
    ui::increment_counter(1);

    // Then
    assert_eq!(ui::total(), 3);
}
```

### [Playwright with `counters_stable`](https://github.com/leptos-rs/leptos/tree/main/examples/counters_stable/e2e)

These tests use the common JavaScript testing tool Playwright to run end-to-end tests on the same example, using a library and testing approach familiar to may who have done frontend development before.

#### Sample Test

```js
import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Increment Count", () => {
  test("should increase the total count", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();
    await ui.addCounter();

    await ui.incrementCount();
    await ui.incrementCount();
    await ui.incrementCount();

    await expect(ui.total).toHaveText("3");
  });
});
```

### [Gherkin/Cucumber Tests with `todo_app_sqlite`](https://github.com/leptos-rs/leptos/blob/main/examples/todo_app_sqlite/e2e/README.md)

You can integrate any testing tool you’d like into this flow. This example uses Cucumber, a testing framework based on natural language.

```
@add_todo
Feature: Add Todo

    Background:
        Given I see the app

    @add_todo-see
    Scenario: Should see the todo
        Given I set the todo as Buy Bread
        When I click the Add button
        Then I see the todo named Buy Bread

    # @allow.skipped
    @add_todo-style
    Scenario: Should see the pending todo
        When I add a todo as Buy Oranges
        Then I see the pending todo
```

The definitions for these actions are defined in Rust code.

```rust
use crate::fixtures::{action, world::AppWorld};
use anyhow::{Ok, Result};
use cucumber::{given, when};

#[given("I see the app")]
#[when("I open the app")]
async fn i_open_the_app(world: &mut AppWorld) -> Result<()> {
    let client = &world.client;
    action::goto_path(client, "").await?;

    Ok(())
}

#[given(regex = "^I add a todo as (.*)$")]
#[when(regex = "^I add a todo as (.*)$")]
async fn i_add_a_todo_titled(world: &mut AppWorld, text: String) -> Result<()> {
    let client = &world.client;
    action::add_todo(client, text.as_str()).await?;

    Ok(())
}

// etc.
```

### Learning More

Feel free to check out the CI setup in the Leptos repo to learn more about how to use these tools in your own application. All of these testing methods are run regularly against actual Leptos example apps.
