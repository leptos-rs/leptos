# Deployment

There are as many ways to deploy a web application as there are developers, let alone applications. But there are a couple useful tips to keep in mind when deploying an app.

## General Advice

1. Remember: Always deploy Rust apps built in `--release` mode, not debug mode. This has a huge effect on both performance and binary size.
2. Test locally in release mode as well. The framework applies certain optimizations in release mode that it does not apply in debug mode, so it’s possible for bugs to surface at this point. (If your app behaves differently or you do encounter a bug, it’s likely a framework-level bug and you should open a GitHub issue with a reproduction.)
3. See the chapter on "Optimizing WASM Binary Size" for additional tips and tricks to further improve the time-to-interactive metric for your WASM app on first load.

> We asked users to submit their deployment setups to help with this chapter. I’ll quote from them below, but you can read the full thread [here](https://github.com/leptos-rs/leptos/issues/1152).

## Deploying a Client-Side-Rendered App

If you’ve been building an app that only uses client-side rendering, working with Trunk as a dev server and build tool, the process is quite easy.

```bash
trunk build --release
```

`trunk build` will create a number of build artifacts in a `dist/` directory. Publishing `dist` somewhere online should be all you need to deploy your app. This should work very similarly to deploying any JavaScript application.

> Read more: [Deploying to Vercel with GitHub Actions](https://github.com/leptos-rs/leptos/issues/1152#issuecomment-1577861900).

## Deploying a Full-Stack App

The most popular way for people to deploy full-stack apps built with `cargo-leptos` is to use a cloud hosting service that supports deployment via a Docker build. Here’s a sample `Dockerfile`, which is based on the one we use to deploy the Leptos website.

```dockerfile
# Get started with a build env with Rust nightly
FROM rustlang/rust:nightly-bullseye as builder

# If you’re using stable, use this instead
# FROM rust:1.70-bullseye as builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

# Build the app
RUN cargo leptos build --release -vv

FROM rustlang/rust:nightly-bullseye as runner
# Copy the server binary to the /app directory
COPY --from=builder /app/target/release/leptos-start /app/
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/
WORKDIR /app

# Set any required env variables and
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080
# Run the server
CMD ["/app/leptos_start"]
```

> Read more: [`gnu` and `musl` build files for Leptos apps](https://github.com/leptos-rs/leptos/issues/1152#issuecomment-1634916088).
