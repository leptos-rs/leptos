# Forms and Inputs

Forms and form inputs are an important part of interactive apps. There are two
basic patterns for interacting with inputs in Leptos, which you may recognize
if you’re familiar with React, SolidJS, or a similar framework: using **controlled**
or **uncontrolled** inputs.

## Controlled Inputs

In a "controlled input," the framework controls the state of the input
element. On every `input` event, it updates a local signal that holds the current
state, which in turn updates the `value` prop of the input.

There are two important things to remember:

1. The `input` event fires on (almost) every change to the element, while the
   `change` event fires (more or less) when you unfocus the input. You probably
   want `on:input`, but we give you the freedom to choose.
2. The `value` _attribute_ only sets the initial value of the input, i.e., it
   only updates the input up to the point that you begin typing. The `value`
   _property_ continues updating the input after that. You usually want to set
   `prop:value` for this reason. (The same is true for `checked` and `prop:checked`
   on an `<input type="checkbox">`.)

```rust
let (name, set_name) = create_signal("Controlled".to_string());

view! {
    <input type="text"
        on:input=move |ev| {
            // event_target_value is a Leptos helper function
            // it functions the same way as event.target.value
            // in JavaScript, but smooths out some of the typecasting
            // necessary to make this work in Rust
            set_name(event_target_value(&ev));
        }

        // the `prop:` syntax lets you update a DOM property,
        // rather than an attribute.
        prop:value=name
    />
    <p>"Name is: " {name}</p>
}
```

> #### Why do you need `prop:value`?
>
> Web browsers are the most ubiquitous and stable platform for rendering graphical user interfaces in existence. They have also maintained an incredible backwards compatibility over their three decades of existence. Inevitably, this means there are some quirks.
>
> One odd quirk is that there is a distinction between HTML attributes and DOM element properties, i.e., between something called an “attribute” which is parsed from HTML and can be set on a DOM element with `.setAttribute()`, and something called a “property” which is a field of the JavaScript class representation of that parsed HTML element.
>
> In the case of an `<input value=...>`, setting the `value` _attribute_ is defined as setting the initial value for the input, and setting `value` _property_ sets its current value. It maybe easiest to understand this by opening `about:blank` and running the following JavaScript in the browser console, line by line:
>
> ```js
> // create an input and append it to the DOM
> const el = document.createElement("input");
> document.body.appendChild(el);
>
> el.setAttribute("value", "test"); // updates the input
> el.setAttribute("value", "another test"); // updates the input again
>
> // now go and type into the input: delete some characters, etc.
>
> el.setAttribute("value", "one more time?");
> // nothing should have changed. setting the "initial value" does nothing now
>
> // however...
> el.value = "But this works";
> ```
>
> Many other frontend frameworks conflate attributes and properties, or create a special case for inputs that sets the value correctly. Maybe Leptos should do this too; but for now, I prefer giving users the maximum amount of control over whether they’re setting an attribute or a property, and doing my best to educate people about the actual underlying browser behavior rather than obscuring it.

## Uncontrolled Inputs

In an "uncontrolled input," the browser controls the state of the input element.
Rather than continuously updating a signal to hold its value, we use a
[`NodeRef`](https://docs.rs/leptos/latest/leptos/struct.NodeRef.html) to access
the input once when we want to get its value.

In this example, we only notify the framework when the `<form>` fires a `submit`
event.

```rust
let (name, set_name) = create_signal("Uncontrolled".to_string());

let input_element: NodeRef<Input> = create_node_ref();
```

`NodeRef` is a kind of reactive smart pointer: we can use it to access the
underlying DOM node. Its value will be set when the element is rendered.

```rust
let on_submit = move |ev: SubmitEvent| {
    // stop the page from reloading!
    ev.prevent_default();

    // here, we'll extract the value from the input
    let value = input_element()
        // event handlers can only fire after the view
        // is mounted to the DOM, so the `NodeRef` will be `Some`
        .expect("<input> to exist")
        // `NodeRef` implements `Deref` for the DOM element type
        // this means we can call`HtmlInputElement::value()`
        // to get the current value of the input
        .value();
    set_name(value);
};
```

Our `on_submit` handler will access the input’s value and use it to call `set_name`.
To access the DOM node stored in the `NodeRef`, we can simply call it as a function
(or using `.get()`). This will return `Option<web_sys::HtmlInputElement>`, but we
know it will already have been filled when we rendered the view, so it’s safe to
unwrap here.

We can then call `.value()` to get the value out of the input, because `NodeRef`
gives us access to a correctly-typed HTML element.

```rust
view! {
    <form on:submit=on_submit>
        <input type="text"
            value=name
            node_ref=input_element
        />
        <input type="submit" value="Submit"/>
    </form>
    <p>"Name is: " {name}</p>
}
```

The view should be pretty self-explanatory by now. Note two things:

1. Unlike in the controlled input example, we use `value` (not `prop:value`).
   This is because we’re just setting the initial value of the input, and letting
   the browser control its state. (We could use `prop:value` instead.)
2. We use `node_ref` to fill the `NodeRef`. (Older examples sometimes use `_ref`.
   They are the same thing, but `node_ref` has better rust-analyzer support.)

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/5-forms-0-5-rf2t7c?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/5-forms-0-5-rf2t7c?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::{ev::SubmitEvent, *};

#[component]
fn App() -> impl IntoView {
    view! {
        <h2>"Controlled Component"</h2>
        <ControlledComponent/>
        <h2>"Uncontrolled Component"</h2>
        <UncontrolledComponent/>
    }
}

#[component]
fn ControlledComponent() -> impl IntoView {
    // create a signal to hold the value
    let (name, set_name) = create_signal("Controlled".to_string());

    view! {
        <input type="text"
            // fire an event whenever the input changes
            on:input=move |ev| {
                // event_target_value is a Leptos helper function
                // it functions the same way as event.target.value
                // in JavaScript, but smooths out some of the typecasting
                // necessary to make this work in Rust
                set_name(event_target_value(&ev));
            }

            // the `prop:` syntax lets you update a DOM property,
            // rather than an attribute.
            //
            // IMPORTANT: the `value` *attribute* only sets the
            // initial value, until you have made a change.
            // The `value` *property* sets the current value.
            // This is a quirk of the DOM; I didn't invent it.
            // Other frameworks gloss this over; I think it's
            // more important to give you access to the browser
            // as it really works.
            //
            // tl;dr: use prop:value for form inputs
            prop:value=name
        />
        <p>"Name is: " {name}</p>
    }
}

#[component]
fn UncontrolledComponent() -> impl IntoView {
    // import the type for <input>
    use leptos::html::Input;

    let (name, set_name) = create_signal("Uncontrolled".to_string());

    // we'll use a NodeRef to store a reference to the input element
    // this will be filled when the element is created
    let input_element: NodeRef<Input> = create_node_ref();

    // fires when the form `submit` event happens
    // this will store the value of the <input> in our signal
    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        // here, we'll extract the value from the input
        let value = input_element()
            // event handlers can only fire after the view
            // is mounted to the DOM, so the `NodeRef` will be `Some`
            .expect("<input> to exist")
            // `NodeRef` implements `Deref` for the DOM element type
            // this means we can call`HtmlInputElement::value()`
            // to get the current value of the input
            .value();
        set_name(value);
    };

    view! {
        <form on:submit=on_submit>
            <input type="text"
                // here, we use the `value` *attribute* to set only
                // the initial value, letting the browser maintain
                // the state after that
                value=name

                // store a reference to this input in `input_element`
                node_ref=input_element
            />
            <input type="submit" value="Submit"/>
        </form>
        <p>"Name is: " {name}</p>
    }
}

// This `main` function is the entry point into the app
// It just mounts our component to the <body>
// Because we defined it as `fn App`, we can now use it in a
// template as <App/>
fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
