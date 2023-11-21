# Part 2: Server Side Rendering

The second part of the book is all about how to turn your beautiful UI's into full-stack, "Universal" Rust + Leptos powered websites and applications.

As you read in the last chapter, there are some limitations to using Client-Side Rendered Leptos apps - over the next few chapters, you'll see how we can overcome those limitations
and get the best performance and SEO out of your Leptos apps.

Let's see how we can use the full power of Leptos and Rust on the server to make your next cutting-edge application!


```admonish info

When working with Leptos on the server side, you're free to choose either the Actix-web or the Axum integrations - the full feature set of Leptos is available with either option.

If, however, you need deploy to a WinterCG-compatible runtime like Deno, Cloudflare, etc., then choose the Axum integration as this deployment option is only available with Axum on the server. Lastly, if you'd like to go full-stack WASM/WASI and deploy to WASM-based serverless runtimes, then Axum is your go-to choice here too.

NB: this is a limitation of the web frameworks themselves, not Leptos.

```