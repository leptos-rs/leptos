This example shows the basics of how you’d render a Leptos component to HTML in a Cloudflare Worker and then hydrate it on the client side.

First, compile the client-side WebAssembly with
```sh
wasm-pack build --target=web --no-default-features --features=hydrate
```

Then run the Worker:

```sh
# run your Worker in an ideal development workflow (with a local server, file watcher & more)
$ npm run dev

# deploy your Worker globally to the Cloudflare network (update your wrangler.toml file for configuration)
$ npm run deploy
```

## Important Note
It's possible the URL for some of the JS necessary for hydration will change between `wasm-pack` builds. Obviously this is not great, but this is a proof of concept more than anything. If there's trouble loading JS, check the URL at `lib.rs:72` against the filenames in `pkg` and adjust `lib.rs` to match the correct path.