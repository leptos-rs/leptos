# Nested Routing

We just defined the following set of routes:

```rust
<Routes>
  <Route path="/" view=Home/>
  <Route path="/users" view=Users/>
  <Route path="/users/:id" view=UserProfile/>
  <Route path="/*any" view=NotFound/>
</Routes>
```

There’s a certain amount of duplication here: `/users` and `/users/:id`. This is fine for a small app, but you can probably already tell it won’t scale well. Wouldn’t it be nice if we could nest these routes?

Well... you can!

```rust
<Routes>
  <Route path="/" view=Home/>
  <Route path="/users" view=Users>
    <Route path=":id" view=UserProfile/>
  </Route>
  <Route path="/*any" view=NotFound/>
</Routes>
```

But wait. We’ve just subtly changed what our application does.

The next section is one of the most important in this entire routing section of the guide. Read it carefully, and feel free to ask questions if there’s anything you don’t understand.

# Nested Routes as Layout

Nested routes are a form of layout, not a method of route definition.

Let me put that another way: The goal of defining nested routes is not primarily to avoid repeating yourself when typing out the paths in your route definitions. It is actually to tell the router to display multiple `<Route/>`s on the page at the same time, side by side.

Let’s look back at our practical example.

```rust
<Routes>
  <Route path="/users" view=Users/>
  <Route path="/users/:id" view=UserProfile/>
</Routes>
```

This means:

- If I go to `/users`, I get the `<Users/>` component.
- If I go to `/users/3`, I get the `<UserProfile/>` component (with the parameter `id` set to `3`; more on that later)

Let’s say I use nested routes instead:

```rust
<Routes>
  <Route path="/users" view=Users>
    <Route path=":id" view=UserProfile/>
  </Route>
</Routes>
```

This means:

- If I go to `/users/3`, the path matches two `<Route/>`s: `<Users/>` and `<UserProfile/>`.
- If I go to `/users`, the path is not matched.

I actually need to add a fallback route

```rust
<Routes>
  <Route path="/users" view=Users>
    <Route path=":id" view=UserProfile/>
    <Route path="" view=NoUser/>
  </Route>
</Routes>
```

Now:

- If I go to `/users/3`, the path matches `<Users/>` and `<UserProfile/>`.
- If I go to `/users`, the path matches `<Users/>` and `<NoUser/>`.

When I use nested routes, in other words, each **path** can match multiple **routes**: each URL can render the views provided by multiple `<Route/>` components, at the same time, on the same page.

This may be counter-intuitive, but it’s very powerful, for reasons you’ll hopefully see in a few minutes.

## Why Nested Routing?

Why bother with this?

Most web applications contain levels of navigation that correspond to different parts of the layout. For example, in an email app you might have a URL like `/contacts/greg`, which shows a list of contacts on the left of the screen, and contact details for Greg on the right of the screen. The contact list and the contact details should always appear on the screen at the same time. If there’s no contact selected, maybe you want to show a little instructional text.

You can easily define this with nested routes

```rust
<Routes>
  <Route path="/contacts" view=ContactList>
    <Route path=":id" view=ContactInfo/>
    <Route path="" view=|| view! {
      <p>"Select a contact to view more info."</p>
    }/>
  </Route>
</Routes>
```

You can go even deeper. Say you want to have tabs for each contact’s address, email/phone, and your conversations with them. You can add _another_ set of nested routes inside `:id`:

```rust
<Routes>
  <Route path="/contacts" view=ContactList>
    <Route path=":id" view=ContactInfo>
      <Route path="" view=EmailAndPhone/>
      <Route path="address" view=Address/>
      <Route path="messages" view=Messages/>
    </Route>
    <Route path="" view=|| view! {
      <p>"Select a contact to view more info."</p>
    }/>
  </Route>
</Routes>
```

> The main page of the [Remix website](https://remix.run/), a React framework from the creators of React Router, has a great visual example if you scroll down, with three levels of nested routing: Sales > Invoices > an invoice.

## `<Outlet/>`

Parent routes do not automatically render their nested routes. After all, they are just components; they don’t know exactly where they should render their children, and “just stick it at the end of the parent component” is not a great answer.

Instead, you tell a parent component where to render any nested components with an `<Outlet/>` component. The `<Outlet/>` simply renders one of two things:

- if there is no nested route that has been matched, it shows nothing
- if there is a nested route that has been matched, it shows its `view`

That’s all! But it’s important to know and to remember, because it’s a common source of “Why isn’t this working?” frustration. If you don’t provide an `<Outlet/>`, the nested route won’t be displayed.

```rust
#[component]
pub fn ContactList() -> impl IntoView {
  let contacts = todo!();

  view! {
    <div style="display: flex">
      // the contact list
      <For each=contacts
        key=|contact| contact.id
        children=|contact| todo!()
      />
      // the nested child, if any
      // don’t forget this!
      <Outlet/>
    </div>
  }
}
```

## Refactoring Route Definitions

You don’t need to define all your routes in one place if you don’t want to. You can refactor any `<Route/>` and its children out into a separate component.

For example, you can refactor the example above to use two separate components:

```rust
#[component]
fn App() -> impl IntoView {
  view! {
    <Router>
      <Routes>
        <Route path="/contacts" view=ContactList>
          <ContactInfoRoutes/>
          <Route path="" view=|| view! {
            <p>"Select a contact to view more info."</p>
          }/>
        </Route>
      </Routes>
    </Router>
  }
}

#[component(transparent)]
fn ContactInfoRoutes() -> impl IntoView {
  view! {
    <Route path=":id" view=ContactInfo>
      <Route path="" view=EmailAndPhone/>
      <Route path="address" view=Address/>
      <Route path="messages" view=Messages/>
    </Route>
  }
}
```

This second component is a `#[component(transparent)]`, meaning it just returns its data, not a view: in this case, it's a [`RouteDefinition`](https://docs.rs/leptos_router/latest/leptos_router/struct.RouteDefinition.html) struct, which is what the `<Route/>` returns. As long as it is marked `#[component(transparent)]`, this sub-route can be defined wherever you want, and inserted as a component into your tree of route definitions.

## Nested Routing and Performance

All of this is nice, conceptually, but again—what’s the big deal?

Performance.

In a fine-grained reactive library like Leptos, it’s always important to do the least amount of rendering work you can. Because we’re working with real DOM nodes and not diffing a virtual DOM, we want to “rerender” components as infrequently as possible. Nested routing makes this extremely easy.

Imagine my contact list example. If I navigate from Greg to Alice to Bob and back to Greg, the contact information needs to change on each navigation. But the `<ContactList/>` should never be rerendered. Not only does this save on rendering performance, it also maintains state in the UI. For example, if I have a search bar at the top of `<ContactList/>`, navigating from Greg to Alice to Bob won’t clear the search.

In fact, in this case, we don’t even need to rerender the `<Contact/>` component when moving between contacts. The router will just reactively update the `:id` parameter as we navigate, allowing us to make fine-grained updates. As we navigate between contacts, we’ll update single text nodes to change the contact’s name, address, and so on, without doing _any_ additional rerendering.

> This sandbox includes a couple features (like nested routing) discussed in this section and the previous one, and a couple we’ll cover in the rest of this chapter. The router is such an integrated system that it makes sense to provide a single example, so don’t be surprised if there’s anything you don’t understand.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/16-router-0-5-4xp4zz?file=%2Fsrc%2Fmain.rs%3A102%2C2)

<iframe src="https://codesandbox.io/p/sandbox/16-router-0-5-4xp4zz?file=%2Fsrc%2Fmain.rs%3A102%2C2" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;
use leptos_router::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <h1>"Contact App"</h1>
            // this <nav> will show on every routes,
            // because it's outside the <Routes/>
            // note: we can just use normal <a> tags
            // and the router will use client-side navigation
            <nav>
                <h2>"Navigation"</h2>
                <a href="/">"Home"</a>
                <a href="/contacts">"Contacts"</a>
            </nav>
            <main>
                <Routes>
                    // / just has an un-nested "Home"
                    <Route path="/" view=|| view! {
                        <h3>"Home"</h3>
                    }/>
                    // /contacts has nested routes
                    <Route
                        path="/contacts"
                        view=ContactList
                      >
                        // if no id specified, fall back
                        <Route path=":id" view=ContactInfo>
                            <Route path="" view=|| view! {
                                <div class="tab">
                                    "(Contact Info)"
                                </div>
                            }/>
                            <Route path="conversations" view=|| view! {
                                <div class="tab">
                                    "(Conversations)"
                                </div>
                            }/>
                        </Route>
                        // if no id specified, fall back
                        <Route path="" view=|| view! {
                            <div class="select-user">
                                "Select a user to view contact info."
                            </div>
                        }/>
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn ContactList() -> impl IntoView {
    view! {
        <div class="contact-list">
            // here's our contact list component itself
            <div class="contact-list-contacts">
                <h3>"Contacts"</h3>
                <A href="alice">"Alice"</A>
                <A href="bob">"Bob"</A>
                <A href="steve">"Steve"</A>
            </div>

            // <Outlet/> will show the nested child route
            // we can position this outlet wherever we want
            // within the layout
            <Outlet/>
        </div>
    }
}

#[component]
fn ContactInfo() -> impl IntoView {
    // we can access the :id param reactively with `use_params_map`
    let params = use_params_map();
    let id = move || params.with(|params| params.get("id").cloned().unwrap_or_default());

    // imagine we're loading data from an API here
    let name = move || match id().as_str() {
        "alice" => "Alice",
        "bob" => "Bob",
        "steve" => "Steve",
        _ => "User not found.",
    };

    view! {
        <div class="contact-info">
            <h4>{name}</h4>
            <div class="tabs">
                <A href="" exact=true>"Contact Info"</A>
                <A href="conversations">"Conversations"</A>
            </div>

            // <Outlet/> here is the tabs that are nested
            // underneath the /contacts/:id route
            <Outlet/>
        </div>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
