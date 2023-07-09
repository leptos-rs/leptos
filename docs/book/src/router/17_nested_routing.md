# Nested Routing

We just defined the following set of routes:

```rust
<Routes>
  <Route path="/" view=|cx| view! { cx, <Home /> }/>
  <Route path="/users" view=|cx| view! { cx, <Users /> }/>
  <Route path="/users/:id" view=|cx| view! { cx, <UserProfile /> }/>
  <Route path="/*any" view=|cx| view! { cx, <NotFound /> }/>
</Routes>
```

There’s a certain amount of duplication here: `/users` and `/users/:id`. This is fine for a small app, but you can probably already tell it won’t scale well. Wouldn’t it be nice if we could nest these routes?

Well... you can!

```rust
<Routes>
  <Route path="/" view=|cx| view! { cx, <Home /> }/>
  <Route path="/users" view=|cx| view! { cx, <Users /> }>
    <Route path=":id" view=|cx| view! { cx, <UserProfile /> }/>
  </Route>
  <Route path="/*any" view=|cx| view! { cx, <NotFound /> }/>
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
  <Route path="/users" view=|cx| view! { cx, <Users /> }/>
  <Route path="/users/:id" view=|cx| view! { cx, <UserProfile /> }/>
</Routes>
```

This means:

- If I go to `/users`, I get the `<Users/>` component.
- If I go to `/users/3`, I get the `<UserProfile/>` component (with the parameter `id` set to `3`; more on that later)

Let’s say I use nested routes instead:

```rust
<Routes>
  <Route path="/users" view=|cx| view! { cx, <Users /> }>
    <Route path=":id" view=|cx| view! { cx, <UserProfile /> }/>
  </Route>
</Routes>
```

This means:

- If I go to `/users/3`, the path matches two `<Route/>`s: `<Users/>` and `<UserProfile/>`.
- If I go to `/users`, the path is not matched.

I actually need to add a fallback route

```rust
<Routes>
  <Route path="/users" view=|cx| view! { cx, <Users /> }>
    <Route path=":id" view=|cx| view! { cx, <UserProfile /> }/>
    <Route path="" view=|cx| view! { cx, <NoUser /> }/>
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
  <Route path="/contacts" view=|cx| view! { cx, <ContactList/> }>
    <Route path=":id" view=|cx| view! { cx, <ContactInfo/> }/>
    <Route path="" view=|cx| view! { cx,
      <p>"Select a contact to view more info."</p>
    }/>
  </Route>
</Routes>
```

You can go even deeper. Say you want to have tabs for each contact’s address, email/phone, and your conversations with them. You can add _another_ set of nested routes inside `:id`:

```rust
<Routes>
  <Route path="/contacts" view=|cx| view! { cx, <ContactList/> }>
    <Route path=":id" view=|cx| view! { cx, <ContactInfo/> }>
      <Route path="" view=|cx| view! { cx, <EmailAndPhone/> }/>
      <Route path="address" view=|cx| view! { cx, <Address/> }/>
      <Route path="messages" view=|cx| view! { cx, <Messages/> }/>
    </Route>
    <Route path="" view=|cx| view! { cx,
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
pub fn ContactList(cx: Scope) -> impl IntoView {
  let contacts = todo!();

  view! { cx,
    <div style="display: flex">
      // the contact list
      <For each=contacts
        key=|contact| contact.id
        view=|cx, contact| todo!()
      >
      // the nested child, if any
      // don’t forget this!
      <Outlet/>
    </div>
  }
}
```

## Nested Routing and Performance

All of this is nice, conceptually, but again—what’s the big deal?

Performance.

In a fine-grained reactive library like Leptos, it’s always important to do the least amount of rendering work you can. Because we’re working with real DOM nodes and not diffing a virtual DOM, we want to “rerender” components as infrequently as possible. Nested routing makes this extremely easy.

Imagine my contact list example. If I navigate from Greg to Alice to Bob and back to Greg, the contact information needs to change on each navigation. But the `<ContactList/>` should never be rerendered. Not only does this save on rendering performance, it also maintains state in the UI. For example, if I have a search bar at the top of `<ContactList/>`, navigating from Greg to Alice to Bob won’t clear the search.

In fact, in this case, we don’t even need to rerender the `<Contact/>` component when moving between contacts. The router will just reactively update the `:id` parameter as we navigate, allowing us to make fine-grained updates. As we navigate between contacts, we’ll update single text nodes to change the contact’s name, address, and so on, without doing _any_ additional rerendering.

> This sandbox includes a couple features (like nested routing) discussed in this section and the previous one, and a couple we’ll cover in the rest of this chapter. The router is such an integrated system that it makes sense to provide a single example, so don’t be surprised if there’s anything you don’t understand.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/16-router-fy4tjv?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A3%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A3%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/16-router-fy4tjv?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A3%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A3%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
