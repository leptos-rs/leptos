# Templating: Building User Interfaces

> The code for this chapter can be found [here](https://github.com/leptos-rs/leptos/tree/main/docs/book/project/ch03_building_ui).

## RSX and the `view!` macro

Okay, that “Hello, world!” was a little boring. We’re going to be building a todo app, so let’s look at something a little more complicated.

As you noticed in the first example, Leptos lets you describe your user interface with a declarative `view!` macro. It looks something like this:

```
view! {
	cx, // this is the "reactive scope": more on that in the next chapter
	<p>"..."</p> // this is some HTML-ish stuff
}
```

The “HTML-ish stuff” is what we call “RSX”: XML in Rust. (You may recognize the similarity to JSX, which is the mixed JavaScript/XML syntax used by frameworks like React.)

Here’s a more in-depth example:

```rust
{{#include ../project/ch03_building_ui/src/main.rs}}
```

You’ll probably notice a few things right away:

1. Elements without children need to be explicit closed with a `/` (`<input/>`, not `<input>`)
2. Text nodes are formatted as strings, i.e., wrapped in quotation marks (`"My Tasks"`)
3. Dynamic blocks can be inserted as children of elements, if wrapped in curly braces (`<h2>"by " {name}</h2>`)
4. Attributes can be given Rust expressions as values. This could be a string literal as in HTML (`<input type="text" .../>)` or a variable or block (`data-user=userid` or `on:click=move |_| { ... }`)
5. Unlike in HTML, whitespace is ignored and should be manually added (it’s `<h2>"by " {name}</h2>`, not `<h2>"by" {name}</h2>`; the space between `"by"` and `{name}` is ignored.)
6. Normal attributes work exactly like you'd think they would.
7. There are also special, prefixed attributes.

- `class:` lets you make targeted updates to a single class
- `on:` lets you add an event listener
- `prop:` lets you set a property on a DOM element
- `_ref` stores the DOM element you’re creating in a variable

> You can find more information in the [reference docs for the `view!` macro](https://docs.rs/leptos/0.0.15/leptos/macro.view.html).

## But, wait...

This example shows some parts of the Leptos templating syntax. But it’s completely static.

How do you actually make the user interface interactive?

In the next chapter, we’ll talk about “fine-grained reactivity,” which is the core of the Leptos framework.
