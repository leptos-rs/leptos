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
2. The `value` *attribute* only sets the initial value of the input, i.e., it 
   only updates the input up to the point that you begin typing. The `value` 
   *property* continues updating the input after that. You usually want to set 
   `prop:value` for this reason.

```rust
let (name, set_name) = create_signal(cx, "Controlled".to_string());

view! { cx,
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

## Uncontrolled Inputs 

In an "uncontrolled input," the browser controls the state of the input element. 
Rather than continuously updating a signal to hold its value, we use a 
[`NodeRef`](https://docs.rs/leptos/latest/leptos/struct.NodeRef.html) to access 
the input once when we want to get its value.

In this example, we only notify the framework when the `<form>` fires a `submit` 
event.

```rust
let (name, set_name) = create_signal(cx, "Uncontrolled".to_string());

let input_element: NodeRef<HtmlElement<Input>> = NodeRef::new(cx);
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
view! { cx,
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

