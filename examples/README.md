# Examples README

## Main Branch

The examples in this directory are all built and tested against the current `main` branch.

To the extent that new features have been released or breaking changes have been made since the previous release, the examples are compatible with the `main` branch and not the current release.

## Getting Started

The simplest way to get started with any example is to use the “quick start” command found in the README for each example. Most of the examples use either [`trunk`](https://trunkrs.dev/) (a simple build system and dev server for client-side-rendered apps) or [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) (a build system for server-rendered and client-hydrated apps).

## Using Cargo Make

You can also run any of the examples using [`cargo-make`](https://github.com/sagiegurari/cargo-make). Note that this is completely optional. We use it for CI, and it can be convenient for running the examples, but is not required.

Follow these steps to get any example up and running.

1. `cd` to the example you want to run
2. Make sure `cargo-make` is installed (for example by running `cargo install cargo-make`)
3. Make sure `rustup target add wasm32-unknown-unknown` was executed for the currently selected toolchain.
4. Run `cargo make ci` to setup and test the example
5. Run `cargo make start` to run the example
6. Open the client URL in the console output (<http://127.0.0.1:8080> or <http://127.0.0.1:3000> by default)
7. Run `cargo make stop` to end any processes started by `cargo make start`.

Here are a few additional notes:

- Extendable custom task files are located in the [cargo-make](./cargo-make/) directory
- Running a task will automatically install `cargo` dependencies
- Each `Makefile.toml` file must extend the [cargo-make/main.toml](./cargo-make/main.toml) file
- [cargo-make](./cargo-make/) files that end in `*-test.toml` configure web testing strategies
- Run `cargo make test-report` to learn which examples have web tests

## Prerequisites

Example projects depend on the following tools. Please install them as needed.

- [Rust](https://www.rust-lang.org/)
- Nightly Rust
  - Run `rustup toolchain install nightly`
  - Run `rustup target add wasm32-unknown-unknown`
- [Cargo Make](https://sagiegurari.github.io/cargo-make/)
  - Run `cargo install --force cargo-make`
  - Setup a command alias like `alias cm='cargo make'` to reduce typing (**_Optional_**)
- [Trunk](https://github.com/thedodd/trunk)
  - Run `cargo install trunk`
- [Node Version Manager](https://github.com/nvm-sh/nvm/) (**_Optional_**)
- [Node.js](https://nodejs.org/)
- [pnpm](https://pnpm.io/) (**_Optional_**)
