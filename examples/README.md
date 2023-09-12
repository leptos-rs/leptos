# Examples README

## Main Branch

The examples in this directory are all built and tested against the current `main` branch.

To the extent that new features have been released or breaking changes have been made since the previous release, the examples are compatible with the `main` branch but not the current release.

To see the examples as they were at the time of the `0.4.9` release, [click here](https://github.com/leptos-rs/leptos/tree/v0.4.9/examples).

## Prerequisites

Example projects depend on the following tools. Please install them as needed.

- [Rust](https://www.rust-lang.org/)
- Nightly Rust
  - Run `rustup toolchain install nightly`
  - Run `rustup target add wasm32-unknown-unknown`
- [Cargo Make](https://sagiegurari.github.io/cargo-make/)
  - Run `cargo install --force cargo-make`
  - Setup a command alias like `alias cm='cargo make'` to reduce typing (**_Optional_**)
- [Node Version Manager](https://github.com/nvm-sh/nvm/) (**_Optional_**)
- [Node.js](https://nodejs.org/)
- [pnpm](https://pnpm.io/) (**_Optional_**)

## Getting Started

Follow these steps to get any example up and running.

1. `cd` to the example root directory
2. Run `cargo make ci` to setup and test the example
3. Run `cargo make start` to run the example
4. Open the client URL in the console output (<http://127.0.0.1:8080> or <http://127.0.0.1:3000> by default)

## Cargo Make

[Cargo Make](https://sagiegurari.github.io/cargo-make/) supports `ci` and common development tasks.

Here are the highlights.

- Extendable custom task files are located in the [cargo-make](./cargo-make/) directory
- Running a task will automatically install `cargo` dependencies 
- Each `Makefile.toml` file must extend the [cargo-make/main.toml](./cargo-make/main.toml) file
- [cargo-make](./cargo-make/) files that end in `*-test.toml` configure end-to-end web testing
- Run `cargo make test-report` to learn which examples have web tests
