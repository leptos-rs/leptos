# Guide: Islands

Leptos 0.5 introduces the new `experimental-islands` feature. This guide will walk through the islands feature and core concepts, while implementing a demo app using the islands architecture.

## The Islands Architecture

The dominant JavaScript frontend frameworks (React, Vue, Svelte, Solid, Angular) all originated as frameworks for building client-rendered single-page apps (SPAs). The initial page load is rendered to HTML, then hydrated, and subsequent navigations are handled directly in the client. (Hence “single page”: everything happens from a single page load from the server, even if there is client-side routing later.) Each of these frameworks later added server-side rendering to improve initial load times, SEO, and user experience.

This means that by default, the entire app is interactive. It also means that the entire app has to be shipped to the client as JavaScript in order to be hydrated. Leptos has followed this same pattern.

> You can read more in the chapters on [server-side rendering](./ssr/22_life_cycle.md).

But it’s also possible to work in the opposite direction. Rather than taking an entirely-interactive app, rendering it to HTML on the server, and then hydrating it in the browser, you can begin with a plain HTML page and add small areas of interactivity. This is the traditional format for any website or app before the 2010s: your browser makes a series of requests to the server and returns the HTML for each new page in response. After the rise of “single-page apps” (SPA), this approach has sometimes become known as a “multi-page app” (MPA) by comparison.

The phrase “islands architecture” has emerged recently to describe the approach of beginning with a “sea” of server-rendered HTML pages, and adding “islands” of interactivity throughout the page.

> ### Additional Reading
>
> The rest of this guide will look at how to use islands with Leptos. For more background on the approach in general, check out some of the articles below:
>
> - Jason Miller, [“Islands Architecture”](https://jasonformat.com/islands-architecture/), Jason Miller
> - Ryan Carniato, [“Islands & Server Components & Resumability, Oh My!”](https://dev.to/this-is-learning/islands-server-components-resumability-oh-my-319d)
> - [“Islands Architectures”](https://www.patterns.dev/posts/islands-architecture) on patterns.dev
> - [Astro Islands](https://docs.astro.build/en/concepts/islands/)

## Activating Islands Mode

Let’s start with a fresh `cargo-leptos` app:

```bash
cargo leptos new --git leptos-rs/start
```

> I’m using Actix because I like it. Feel free to use Axum; there should be approximately no server-specific differences in this guide.

I’m just going to run

```bash
cargo leptos build
```

in the background while I fire up my editor and keep writing.

The first thing I’ll do is to add the `experimental-islands` feature in my `Cargo.toml`. I need to add this to both `leptos` and `leptos_actix`:

```toml
leptos = { version = "0.5", features = ["nightly", "experimental-islands"] }
leptos_actix = { version = "0.5", optional = true, features = [
  "experimental-islands",
] }
```

Next I’m going to modify the `hydrate` function exported from `src/lib.rs`. I’m going to remove the line that calls `leptos::mount_to_body(App)` and replace it with

```rust
leptos::leptos_dom::HydrationCtx::stop_hydrating();
```

Each “island” we create will actually act as its own entrypoint, so our `hydrate()` function just says “okay, hydration’s done now.”

Okay, now fire up your `cargo leptos watch` and go to [`http://localhost:3000`](http://localhost:3000) (or wherever).

Click the button, and...

Nothing happens!

Perfect.

## Using Islands

Nothing happens because we’ve just totally inverted the mental model of our app. Rather than being interactive by default and hydrating everything, the app is now plain HTML by default, and we need to opt into interactivity.

This has a big effect on WASM binary sizes: if I compile in release mode, this app is a measly 24kb of WASM (uncompressed), compared to 355kb in non-islands mode. (355kb is quite large for a “Hello, world!” It’s really just all the code related to client-side routing, which isn’t being used in the demo.)

When we click the button, nothing happens, because our whole page is static.

So how do we make something happen?

Let’s turn the `HomePage` component into an island!

Here was the non-interactive version:

```rust
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
```

Here’s the interactive version:

```rust
#[island]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
```

Now when I click the button, it works!

The `#[island]` macro works exactly like the `#[component]` macro, except that in islands mode, it designates this as an interactive island. If we check the binary size again, this is 166kb uncompressed in release mode; much larger than the 24kb totally static version, but much smaller than the 355kb fully-hydrated version.

If you open up the source for the page now, you’ll see that your `HomePage` island has been rendered as a special `<leptos-island>` HTML element which specifies which component should be used to hydrate it:

```html
<leptos-island data-component="HomePage" data-hkc="0-0-0">
  <h1 data-hk="0-0-2">Welcome to Leptos!</h1>
  <button data-hk="0-0-3">
    Click Me:
    <!-- <DynChild> -->11<!-- </DynChild> -->
  </button>
</leptos-island>
```

The typical Leptos hydration keys and markers are only present inside the island, only the island is hydrated.

## Using Islands Effectively

Remember that _only_ code within an `#[island]` needs to be compiled to WASM and shipped to the browser. This means that islands should be as small and specific as possible. My `HomePage`, for example, would be better broken apart into a regular component and an island:

```rust
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Welcome to Leptos!"</h1>
        <Counter/>
    }
}

#[island]
fn Counter() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
```

Now the `<h1>` doesn’t need to be included in the client bundle, or hydrated. This seems like a silly distinction now; but note that you can now add as much inert HTML content as you want to the `HomePage` itself, and the WASM binary size will remain exactly the same.

In regular hydration mode, your WASM binary size grows as a function of the size/complexity of your app. In islands mode, your WASM binary grows as a function of the amount of interactivity in your app. You can add as much non-interactive content as you want, outside islands, and it will not increase that binary size.

## Unlocking Superpowers

So, this 50% reduction in WASM binary size is nice. But really, what’s the point?

The point comes when you combine two key facts:

1. Code inside `#[component]` functions now _only_ runs on the server.
2. Children and props can be passed from the server to islands, without being included in the WASM binary.

This means you can run server-only code directly in the body of a component, and pass it directly into the children. Certain tasks that take a complex blend of server functions and Suspense in fully-hydrated apps can be done inline in islands.

We’re going to rely on a third fact in the rest of this demo:

3. Context can be passed between otherwise-independent islands.

So, instead of our counter demo, let’s make something a little more fun: a tabbed interface that reads data from files on the server.

## Passing Server Children to Islands

One of the most powerful things about islands is that you can pass server-rendered children into an island, without the island needing to know anything about them. Islands hydrate their own content, but not children that are passed to them.

As Dan Abramov of React put it (in the very similar context of RSCs), islands aren’t really islands: they’re donuts. You can pass server-only content directly into the “donut hole,” as it were, allowing you to create tiny atolls of interactivity, surrounded on _both_ sides by the sea of inert server HTML.

> In the demo code included below, I added some styles to show all server content as a light-blue “sea,” and all islands as light-green “land.” Hopefully that will help picture what I’m talking about!

To continue with the demo: I’m going to create a `Tabs` component. Switching between tabs will require some interactivity, so of course this will be an island. Let’s start simple for now:

```rust
#[island]
fn Tabs(labels: Vec<String>) -> impl IntoView {
    let buttons = labels
        .into_iter()
        .map(|label| view! { <button>{label}</button> })
        .collect_view();
    view! {
        <div style="display: flex; width: 100%; justify-content: space-between;">
            {buttons}
        </div>
    }
}
```

Oops. This gives me an error

```
error[E0463]: can't find crate for `serde`
  --> src/app.rs:43:1
   |
43 | #[island]
   | ^^^^^^^^^ can't find crate
```

Easy fix: let’s `cargo add serde --features=derive`. The `#[island]` macro wants to pull in `serde` here because it needs to serialize and deserialize the `labels` prop.

Now let’s update the `HomePage` to use `Tabs`.

```rust
#[component]
fn HomePage() -> impl IntoView {
	// these are the files we’re going to read
    let files = ["a.txt", "b.txt", "c.txt"];
	// the tab labels will just be the file names
	let labels = files.iter().copied().map(Into::into).collect();
    view! {
        <h1>"Welcome to Leptos!"</h1>
        <p>"Click any of the tabs below to read a recipe."</p>
        <Tabs labels/>
    }
}
```

If you take a look in the DOM inspector, you’ll see the island is now something like

```html
<leptos-island
  data-component="Tabs"
  data-hkc="0-0-0"
  data-props='{"labels":["a.txt","b.txt","c.txt"]}'
></leptos-island>
```

Our `labels` prop is getting serialized to JSON and stored in an HTML attribute so it can be used to hydrate the island.

Now let’s add some tabs. For the moment, a `Tab` island will be really simple:

```rust
#[island]
fn Tab(index: usize, children: Children) -> impl IntoView {
    view! {
        <div>{children()}</div>
    }
}
```

Each tab, for now will just be a `<div>` wrapping its children.

Our `Tabs` component will also get some children: for now, let’s just show them all.

```rust
#[island]
fn Tabs(labels: Vec<String>, children: Children) -> impl IntoView {
    let buttons = labels
        .into_iter()
        .map(|label| view! { <button>{label}</button> })
        .collect_view();
    view! {
        <div style="display: flex; width: 100%; justify-content: space-around;">
            {buttons}
        </div>
        {children()}
    }
}
```

Okay, now let’s go back into the `HomePage`. We’re going to create the list of tabs to put into our tab box.

```rust
#[component]
fn HomePage() -> impl IntoView {
    let files = ["a.txt", "b.txt", "c.txt"];
    let labels = files.iter().copied().map(Into::into).collect();
	let tabs = move || {
        files
            .into_iter()
            .enumerate()
            .map(|(index, filename)| {
                let content = std::fs::read_to_string(filename).unwrap();
                view! {
                    <Tab index>
                        <h2>{filename.to_string()}</h2>
                        <p>{content}</p>
                    </Tab>
                }
            })
            .collect_view()
    };

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <p>"Click any of the tabs below to read a recipe."</p>
        <Tabs labels>
            <div>{tabs()}</div>
        </Tabs>
    }
}
```

Uh... What?

If you’re used to using Leptos, you know that you just can’t do this. All code in the body of components has to run on the server (to be rendered to HTML) and in the browser (to hydrate), so you can’t just call `std::fs`; it will panic, because there’s no access to the local filesystem (and certainly not to the server filesystem!) in the browser. This would be a security nightmare!

Except... wait. We’re in islands mode. This `HomePage` component _really does_ only run on the server. So we can, in fact, just use ordinary server code like this.

> **Is this a dumb example?** Yes! Synchronously reading from three different local files in a `.map()` is not a good choice in real life. The point here is just to demonstrate that this is, definitely, server-only content.

Go ahead and create three files in the root of the project called `a.txt`, `b.txt`, and `c.txt`, and fill them in with whatever content you’d like.

Refresh the page and you should see the content in the browser. Edit the files and refresh again; it will be updated.

You can pass server-only content from a `#[component]` into the children of an `#[island]`, without the island needing to know anything about how to access that data or render that content.

**This is really important.** Passing server `children` to islands means that you can keep islands small. Ideally, you don’t want to slap and `#[island]` around a whole chunk of your page. You want to break that chunk out into an interactive piece, which can be an `#[island]`, and a bunch of additional server content that can be passed to that island as `children`, so that the non-interactive subsections of an interactive part of the page can be kept out of the WASM binary.

## Passing Context Between Islands

These aren’t really “tabs” yet: they just show every tab, all the time. So let’s add some simple logic to our `Tabs` and `Tab` components.

We’ll modify `Tabs` to create a simple `selected` signal. We provide the read half via context, and set the value of the signal whenever someone clicks one of our buttons.

```rust
#[island]
fn Tabs(labels: Vec<String>, children: Children) -> impl IntoView {
    let (selected, set_selected) = create_signal(0);
    provide_context(selected);

    let buttons = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| view! {
            <button on:click=move |_| set_selected(index)>
                {label}
            </button>
        })
        .collect_view();
// ...
```

And let’s modify the `Tab` island to use that context to show or hide itself:

```rust
#[island]
fn Tab(children: Children) -> impl IntoView {
    let selected = expect_context::<ReadSignal<usize>>();
    view! {
        <div style:display=move || if selected() {
            "block"
        } else {
            "none"
        }>
// ...
```

Now the tabs behave exactly as I’d expect. `Tabs` passes the signal via context to each `Tab`, which uses it to determine whether it should be open or not.

> That’s why in `HomePage`, I made `let tabs = move ||` a function, and called it like `{tabs()}`: creating the tabs lazily this way meant that the `Tabs` island would already have provided the `selected` context by the time each `Tab` went looking for it.

Our complete tabs demo is about 220kb uncompressed: not the smallest demo in the world, but still about a third smaller than the counter button! Just for kicks, I built the same demo without islands mode, using `#[server]` functions and `Suspense`. and it was 429kb. So again, this was about a 50% savings in binary size. And this app includes quite minimal server-only content: remember that as we add additional server-only components and pages, this 220 will not grow.

## Overview

This demo may seem pretty basic. It is. But there are a number of immediate takeaways:

- **50% WASM binary size reduction**, which means measurable improvements in time to interactivity and initial load times for clients.
- **Reduced HTML page size.** This one is less obvious, but it’s true and important: HTML generated from `#[component]`s doesn’t need all the hydration IDs and other boilerplate added.
- **Reduced data serialization costs.** Creating a resource and reading it on the client means you need to serialize the data, so it can be used for hydration. If you’ve also read that data to create HTML in a `Suspense`, you end up with “double data,” i.e., the same exact data is both rendered to HTML and serialized as JSON, increasing the size of responses, and therefore slowing them down.
- **Easily use server-only APIs** inside a `#[component]` as if it were a normal, native Rust function running on the server—which, in islands mode, it is!
- **Reduced `#[server]`/`create_resource`/`Suspense` boilerplate** for loading server data.

## Future Exploration

The `experimental-islands` feature included in 0.5 reflects work at the cutting edge of what frontend web frameworks are exploring right now. As it stands, our islands approach is very similar to Astro (before its recent View Transitions support): it allows you to build a traditional server-rendered, multi-page app and pretty seamlessly integrate islands of interactivity.

There are some small improvements that will be easy to add. For example, we can do something very much like Astro's View Transitions approach:

- add client-side routing for islands apps by fetching subsequent navigations from the server and replacing the HTML document with the new one
- add animated transitions between the old and new document using the View Transitions API
- support explicit persistent islands, i.e., islands that you can mark with unique IDs (something like `persist:searchbar` on the component in the view), which can be copied over from the old to the new document without losing their current state

There are other, larger architectural changes that I’m [not sold on yet](https://github.com/leptos-rs/leptos/issues/1830).

## Additional Information

Check out the [islands PR](https://github.com/leptos-rs/leptos/pull/1660), [roadmap](https://github.com/leptos-rs/leptos/issues/1830), and [Hackernews demo](https://github.com/leptos-rs/leptos/tree/main/examples/hackernews_islands_axum) for additional discussion.

## Demo Code

```rust
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main style="background-color: lightblue; padding: 10px">
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let files = ["a.txt", "b.txt", "c.txt"];
    let labels = files.iter().copied().map(Into::into).collect();
    let tabs = move || {
        files
            .into_iter()
            .enumerate()
            .map(|(index, filename)| {
                let content = std::fs::read_to_string(filename).unwrap();
                view! {
                    <Tab index>
                        <div style="background-color: lightblue; padding: 10px">
                            <h2>{filename.to_string()}</h2>
                            <p>{content}</p>
                        </div>
                    </Tab>
                }
            })
            .collect_view()
    };

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <p>"Click any of the tabs below to read a recipe."</p>
        <Tabs labels>
            <div>{tabs()}</div>
        </Tabs>
    }
}

#[island]
fn Tabs(labels: Vec<String>, children: Children) -> impl IntoView {
    let (selected, set_selected) = create_signal(0);
    provide_context(selected);

    let buttons = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            view! {
                <button on:click=move |_| set_selected(index)>
                    {label}
                </button>
            }
        })
        .collect_view();
    view! {
        <div
            style="display: flex; width: 100%; justify-content: space-around;\
            background-color: lightgreen; padding: 10px;"
        >
            {buttons}
        </div>
        {children()}
    }
}

#[island]
fn Tab(index: usize, children: Children) -> impl IntoView {
    let selected = expect_context::<ReadSignal<usize>>();
    view! {
        <div
            style:background-color="lightgreen"
            style:padding="10px"
            style:display=move || if selected() == index {
                "block"
            } else {
                "none"
            }
        >
            {children()}
        </div>
    }
}
```
