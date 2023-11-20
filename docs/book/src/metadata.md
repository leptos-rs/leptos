# Metadata

So far, everything we’ve rendered has been inside the `<body>` of the HTML document. And this makes sense. After all, everything you can see on a web page lives inside the `<body>`.

However, there are plenty of occasions where you might want to update something inside the `<head>` of the document using the same reactive primitives and component patterns you use for your UI.

That’s where the [`leptos_meta`](https://docs.rs/leptos_meta/latest/leptos_meta/) package comes in.

## Metadata Components

`leptos_meta` provides special components that let you inject data from inside components anywhere in your application into the `<head>`:

[`<Title/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Title.html) allows you to set the document’s title from any component. It also takes a `formatter` function that can be used to apply the same format to the title set by other pages. So, for example, if you put `<Title formatter=|text| format!("{text} — My Awesome Site")/>` in your `<App/>` component, and then `<Title text="Page 1"/>` and `<Title text="Page 2"/>` on your routes, you’ll get `Page 1 — My Awesome Site` and `Page 2 — My Awesome Site`.

[`<Link/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Link.html) takes the standard attributes of the `<link>` element.

[`<Stylesheet/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Stylesheet.html) creates a `<link rel="stylesheet">` with the `href` you give.

[`<Style/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Style.html) creates a `<style>` with the children you pass in (usually a string). You can use this to import some custom CSS from another file at compile time `<Style>{include_str!("my_route.css")}</Style>`.

[`<Meta/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Meta.html) lets you set `<meta>` tags with descriptions and other metadata.

## `<Script/>` and `<script>`

`leptos_meta` also provides a [`<Script/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Script.html) component, and it’s worth pausing here for a second. All of the other components we’ve considered inject `<head>`-only elements in the `<head>`. But a `<script>` can also be included in the body.

There’s a very simple way to determine whether you should use a capital-S `<Script/>` component or a lowercase-s `<script>` element: the `<Script/>` component will be rendered in the `<head>`, and the `<script>` element will be rendered wherever in the `<body>` of your user interface you put it in, alongside other normal HTML elements. These cause JavaScript to load and run at different times, so use whichever is appropriate to your needs.

## `<Body/>` and `<Html/>`

There are even a couple elements designed to make semantic HTML and styling easier. [`<Html/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Html.html) lets you set the `lang` and `dir` on your `<html>` tag from your application code. `<Html/>` and [`<Body/>`](https://docs.rs/leptos_meta/latest/leptos_meta/fn.Body.html) both have `class` props that let you set their respective `class` attributes, which is sometimes needed by CSS frameworks for styling.

`<Body/>` and `<Html/>` both also have `attributes` props which can be used to set any number of additional attributes on them via the `attr:` syntax:

```rust
<Html
	lang="he"
	dir="rtl"
	attr:data-theme="dark"
/>
```

## Metadata and Server Rendering

Now, some of this is useful in any scenario, but some of it is especially important for search-engine optimization (SEO). Making sure you have things like appropriate `<title>` and `<meta>` tags is crucial. Modern search engine crawlers do handle client-side rendering, i.e., apps that are shipped as an empty `index.html` and rendered entirely in JS/WASM. But they prefer to receive pages in which your app has been rendered to actual HTML, with metadata in the `<head>`.

This is exactly what `leptos_meta` is for. And in fact, during server rendering, this is exactly what it does: collect all the `<head>` content you’ve declared by using its components throughout your application, and then inject it into the actual `<head>`.

But I’m getting ahead of myself. We haven’t actually talked about server-side rendering yet. As a matter of fact... Let’s do that next!
