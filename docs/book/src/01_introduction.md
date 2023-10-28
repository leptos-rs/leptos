# Introduction

This book is intended as an introduction to the [Leptos](https://github.com/leptos-rs/leptos) Web framework.
It will walk through the fundamental concepts you need to build applications,
beginning with a simple application rendered in the browser, and building toward a
full-stack application with server-side rendering and hydration.

The guide doesn’t assume you know anything about fine-grained reactivity or the
details of modern Web frameworks. It does assume you are familiar with the Rust
programming language, HTML, CSS, and the DOM and basic Web APIs.

Leptos is most similar to frameworks like [Solid](https://www.solidjs.com) (JavaScript)
and [Sycamore](https://sycamore-rs.netlify.app/) (Rust). There are some similarities
to other frameworks like React (JavaScript), Svelte (JavaScript), Yew (Rust), and
Dioxus (Rust), so knowledge of one of those frameworks may also make it easier to
understand Leptos.

You can find more detailed docs for each part of the API at [Docs.rs](https://docs.rs/leptos/latest/leptos/).

**Important Note**: This current version of the book reflects the `0.5.1` release. The CodeSandbox versions of the examples still reflect `0.4` and earlier APIs and are in the process of being updated.

> The source code for the book is available [here](https://github.com/leptos-rs/leptos/tree/main/docs/book). PRs for typos or clarification are always welcome.
