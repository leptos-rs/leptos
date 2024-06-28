# Leptos Hacker News Example with Axum

This example creates a basic clone of the Hacker News site. It showcases Leptos' ability to:
- Create a client-side rendered app
- Create a server side rendered app with hydration
- Precompress static assets and bundle those in with the server binary

This repo differs from the main Hacker News example by using Axum as it's server, precompressing and embedding static assets into the binary, and dynamically compressing the generated HTML.

## Getting Started

See the [Examples README](../README.md) for setup and run instructions.

## Quick Start

Run `cargo leptos watch --release -P` to run this example.
