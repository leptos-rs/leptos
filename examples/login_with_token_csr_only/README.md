# Leptos Login Example

This example demonstrates a scenario of a client-side rendered application
that uses an existing API that you cannot or do not want to change.
The authentications of this example are done using an API token.

## Run

First start the example server:

```
cd server/ && cargo run
```

then use [`trunk`](https://trunkrs.dev) to serve the SPA:

```
cd client/ && trunk serve
```

finally you can visit the web application at `http://localhost:8080`

The `api-boundary` crate contains data structures that are used by the server and the client.
