# Hot Patching with `dx` 

This is an experimental example exploring how to combine Leptos with the binary hot-patching provided by Dioxus's `subsecond` library and `dx` cli.

### Serving Your App

This requires installing the Dioxus CLI. Until the stable 0.7.0 release, I'd suggest installing from git:

```sh
cargo install dioxus-cli --git https://github.com/DioxusLabs/dioxus 
```

Then you can run the example with `dx serve --hot-patch --platform web`.

### Hot Patching

Changes to the `App` function should be reflected in your app without a full rebuild and reload.

### Limitatations

Currently we only support hot-patching for
- `set_interval` (my initial experiment)
- event listeners/callbacks
- reactive view functions

You probably want to use `AnyView` (via `.into_any()`) on any views that will be hot-patched, so they can be rebuilt correctly despite their types changing when the structure of the view tree changes.

Note that any hot-patch will cause all render effects to run again.

**This is an experiment/POC. I'm not close to done working on it, and none of these limitations are permanent or insurmountable.**
