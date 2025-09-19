# Hot Patching with `dx`

This is an experimental example exploring how to combine Leptos with the binary hot-patching provided by Dioxus's `subsecond` library and `dx` cli.

### Serving Your App

This requires installing the Dioxus CLI version 0.7.0. At the time I'm writing this README, that does not yet have a stable release. Once `dioxus-cli` 0.7.0 has been released, you should use the latest stable release. Until then, I'd suggest installing from git:

```sh
cargo install dioxus-cli --git https://github.com/DioxusLabs/dioxus
```

Then you can run the example with `dx serve --hot-patch --platform web`.

### Hot Patching

Changes to the your application should be reflected in your app without a full rebuild and reload.

### Limitatations

Currently we only support hot-patching for reactive view functions. You probably want to use `AnyView` (via `.into_any()`) on any views that will be hot-patched, so they can be rebuilt correctly despite their types changing when the structure of the view tree changes.

If you are using `leptos_router` this actually works quite well, as every route’s view is erased to `AnyView` and the router itself is a reactive view function: in other words, changes inside any route should be hot-patched in any case.

Note that any hot-patch will cause all render effects to run again. This means that some client-side state (like the values of signals) will be wiped out.

### Build Tooling

The preference of the Dioxus team is that all hot-patching work that uses their `subsecond` also use `dioxus-cli`. As this demo shows, it's completely possible to use `dioxus-cli` to build and run a Leptos project. We do not plan to build `subsecond` into our own build tooling at this time.

**This is an experiment/POC. It is being published because members of the community have found it useful and have asked for the support to be merged in its current state. Further development and bugfixes are a relatively low priority at this time.**
