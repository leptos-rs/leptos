# Routing

## The Basics

Routing drives most websites. A router is the answer to the question, “Given this URL, what should appear on the page?”

A URL consists of many parts. For example, the URL `https://my-cool-blog.com/blog/search?q=Search#results` consists of

- a _scheme_: `https`
- a _domain_: `my-cool-blog.com`
- a **path**: `/blog/search`
- a **query** (or **search**): `?q=Search`
- a _hash_: `#results`

The Leptos Router works with the path and query (`/blog/search?q=Search`). Given this piece of the URL, what should the app render on the page?

## The Philosophy

In most cases, the path should drive what is displayed on the page. From the user’s perspective, for most applications, most major changes in the state of the app should be reflected in the URL. If you copy and paste the URL and open it in another tab, you should find yourself more or less in the same place.

In this sense, the router is really at the heart of the global state management for your application. More than anything else, it drives what is displayed on the page.

The router handles most of this work for you by mapping the current location to particular components.
