# Templating: Building User Interfaces

## Views

Leptos uses a simple `view` macro to create the user interface. If you’re familiar with JSX, then

## Components

**Components** are the basic building blocks of your application. Each component is simply a function that creates DOM nodes and sets up the reactive system that will update them. The component function runs exactly once per instance of the component.

The `component` macro annotates a function as a component, allowing you to use it within other components.

```rust
use leptos::*;

#[component]
fn Button(cx: Scope, text: &'static str) -> Element {
    view! { cx,
        <button>{text}</button>
    }
}

#[component]
fn BoringButtons(cx: Scope) -> Element {
    view! { cx,
        <div>
			<Button text="These"/>
			<Button text="Do"/>
			<Button text="Nothing"/>
		</div>
    }
}
```

## Views

Leptos uses a simple `view` macro to create the user interface. It’s much like HTML, with the following differences:

1. Text within elements follows the rules of normal Rust strings (i.e., quotation marks or other string syntax)

```rust
view! { cx,  <p>"Hello, world!"</p> }
```

2. Values can be inserted between curly braces. Reactive values

```rust
view! { cx,  <p id={non_reactive_variable}>{move || value()}</p> }
```
