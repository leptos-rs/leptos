FROM rustlang/rust:nightly-bullseye as builder
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
#RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
#RUN cp cargo-binstall /usr/local/cargo/bin 
#RUN cargo binstall cargo-leptos -y
#RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
#RUN rustup target add wasm32-unknown-unknown
RUN mkdir -p /app
WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features --features=ssr
RUN ls -l /app/target

FROM rustlang/rust:nightly-bullseye as runner
COPY --from=builder /app/target/release/hackernews /app/
COPY --from=builder /app/pkg /app
COPY --from=builder /app/Cargo.toml /app/
WORKDIR /app
ENV RUST_LOG="info"
ENV LEPTOS_OUTPUT_NAME="hackernews"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080
CMD ["/app/hackernews"]