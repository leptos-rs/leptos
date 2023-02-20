# Working with `async`

So far we’ve only been working with synchronous users interfaces: You provide some input,
the app immediately process it and updates the interface. This is great, but is a tiny
subset of what web applications do. In particular, most web apps have to deal with some kind
of asynchronous data loading, usually loading something from an API.

Asynchronous data is notoriously hard to integrate with the synchronous parts of your code.
In this chapter, we’ll see how Leptos helps smooth out that process for you.
