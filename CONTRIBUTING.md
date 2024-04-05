# Contributing to Leptos

Thanks for your interesting in contributing to Leptos! This is a truly
community-driven framework, and while we have a central maintainer (@gbj)
large parts of the renderer, reactive system, and server integrations have
all been written by other contributors. Contributions are always welcome.

Participation in this community is governed by a [Code of Conduct](./CODE_OF_CONDUCT.md).
Some of the most active conversations around development take place on our
[Discord server](https://discord.gg/YdRAhS7eQB).

This guide seeks to

- describe some of the framework’s values (in a technical, not an ethical, sense)
- provide a high-level overview of how the pieces of the framework fit together
- orient you to the organization of this repository

## Values

Leptos, as a framework, reflects certain technical values:

- **Expose primitives rather than imposing patterns.** Provide building blocks
  that users can combine together to build up more complex behavior, rather than
  requiring users follow certain templates, file formats, etc. e.g., components
  are defined as functions, rather than a bespoke single-file component format.
  The reactive system feeds into the rendering system, rather than being defined
  by it.
- **Bottom-up over top-down.** If you envision a user’s application as a tree
  (like an HTML document), push meaning toward the leaves of the tree. e.g., If data
  needs to be loaded, load it in a granular primitive (resources) rather than a
  route- or page-level data structure.
- **Performance by default.** When possible, users should only pay for what they
  use. e.g., we don’t make all component props reactive by default. This is
  because doing so would force the overhead of a reactive prop onto props that don’t
  need to be reactive.
- **Full-stack performance.** Performance can’t be limited to a single metric,
  whether that’s a DOM rendering benchmark, WASM binary size, or server response
  time. Use methods like HTTP streaming and progressive enhancement to enable
  applications to load, become interactive, and respond as quickly as possible.
- **Use safe Rust.** There’s no need for `unsafe` Rust in the framework, and
  avoiding it at all costs reduces the maintenance and testing burden significantly.
- **Embrace Rust semantics.** Especially in things like UI templating, use Rust
  semantics or extend them in a predictable way with control-flow components
  rather than overloading the meaning of Rust terms like `if` or `for` in a
  framework-specific way.
- **Enhance ergonomics without obfuscating what’s happening.** This is by far
  the hardest to achieve. It’s often the case that adding additional layers to
  improve DX (like a custom build tool and starter templates) comes across as
  “too magic” to some people who haven’t had to build the same things manually.
  When possible, make it easier to see how the pieces fit together, without
  sacrificing the improved DX.

## Processes

We do not have PR templates or formal processes for approving PRs. But there
are a few guidelines that will make it a better experience for everyone:

- Run `cargo fmt` before submitting your code.
- Keep PRs limited to addressing one feature or one issue, in general. In some
  cases (e.g., “reduce allocations in the reactive system”) this may touch a number
  of different areas, but is still conceptually one thing.
- If it’s an unsolicited PR not linked to an open issue, please include a
  specific explanation for what it’s trying to achieve. For example: “When I
  was trying to deploy my app under _circumstances X_, I found that the way
  _function Y_ was implemented caused _issue Z_. This PR should fix that by
  _solution._”
- Our CI tests every PR against all the existing examples, sometimes requiring
  compilation for both server and client side, etc. It’s thorough but slow. If
  you want to run CI locally to reduce frustration, you can do that by installing
  `cargo-make` and using `cargo make check && cargo make test && cargo make
check-examples`.

## Before Submitting a PR

We have a fairly extensive CI setup that runs both lints (like `rustfmt` and `clippy`)
and tests on PRs. You can run most of these locally if you have `cargo-make` installed.

Note that some of the `rustfmt` settings used require usage of the nightly compiler.
Formatting the code using the stable toolchain may result in a wrong code format and
subsequently CI errors.
Run `cargo +nightly fmt` if you want to keep the stable toolchain active.
You may want to let your IDE automatically use the `+nightly` parameter when a
"format on save" action is used.

If you added an example, make sure to add it to the list in `examples/Makefile.toml`.

From the root directory of the repo, run
- `cargo +nightly fmt`
- `cargo +nightly make check`
- `cargo +nightly make test`
- `cargo +nightly make check-examples`
- `cargo +nightly make --profile=github-actions ci`

If you modified an example:
- `cd examples/your_example`
- `cargo +nightly fmt -- --config-path ../..`
- `cargo +nightly make --profile=github-actions verify-flow`

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md).
