# Leptos Development Guide

Complete guide for developing with and contributing to the Leptos web framework.

## Prerequisites

### System Requirements
- **Rust**: 1.88+ (workspace MSRV)
- **Node.js**: 18+ (for examples with JS dependencies)
- **wasm-pack**: For WASM compilation
- **cargo-leptos**: Official build tool

### Installation

**Install Rust (via rustup):**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Install cargo-leptos:**
```bash
cargo install cargo-leptos
```

**Install wasm-pack:**
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### Tooling

**Recommended VS Code Extensions:**
- rust-analyzer
- Better TOML
- CodeLLDB (for debugging)

**Recommended CLI Tools:**
```bash
# For better error messages
cargo install cargo-expand

# For testing
cargo install cargo-nextest

# For security audits
cargo install cargo-audit

# For dependency analysis
cargo install cargo-tree
```

## Project Structure

### Workspace Overview
```
leptos/
├── Cargo.toml              # Workspace configuration
├── leptos/                 # Main framework crate
├── leptos_dom/             # DOM utilities
├── leptos_router/          # Routing system
├── leptos_server/          # Server function integration
├── leptos_macro/           # Procedural macros
├── reactive_graph/         # Reactive system core
├── server_fn/             # Server function primitives
├── tachys/                # Rendering engine
├── integrations/          # Server integrations
│   ├── axum/             # Axum integration
│   └── actix/            # Actix Web integration
├── examples/             # Example applications
├── benchmarks/           # Performance benchmarks
└── docs/                # Documentation
```

### Core Dependencies
- **Server Integration**: Axum or Actix Web
- **Serialization**: Serde, Rkyv
- **Async Runtime**: Tokio (server), wasm-bindgen-futures (client)
- **HTTP**: Reqwest (client), HTTP crates (server)

## Development Workflow

### Getting Started

**Clone the repository:**
```bash
git clone https://github.com/leptos-rs/leptos.git
cd leptos
```

**Build the workspace:**
```bash
cargo build --workspace
```

**Run tests:**
```bash
cargo test --workspace
```

### Working with Examples

**List available examples:**
```bash
ls examples/
```

**Run a client-side rendered example:**
```bash
cd examples/counter
trunk serve --open
```

**Run a full-stack example:**
```bash
cd examples/todo_app_sqlite_axum
cargo leptos watch
```

**Available example categories:**
- **Basic**: counter, counters, timer
- **Advanced**: todo_app_sqlite, hackernews
- **SSR**: ssr_modes, hackernews_axum
- **Styling**: tailwind_axum, tailwind_csr
- **Testing**: suspense_tests, regression

### Development Commands

**Format code:**
```bash
cargo fmt --all
```

**Check for issues:**
```bash
cargo clippy --workspace --all-targets --all-features
```

**Run specific tests:**
```bash
# Test specific crate
cargo test -p leptos

# Test with features
cargo test --features "ssr"

# Run WASM tests
wasm-pack test --headless --chrome leptos_dom
```

**Benchmark performance:**
```bash
cd benchmarks
cargo bench
```

## Feature Development

### Creating Components

**Basic component structure:**
```rust
use leptos::*;

#[component]
pub fn MyComponent(
    #[prop(default = "default".into())] 
    title: String,
    #[prop(optional)] 
    subtitle: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="my-component">
            <h1>{title}</h1>
            {subtitle.map(|s| view! { <h2>{s}</h2> })}
            <div class="content">
                {children()}
            </div>
        </div>
    }
}
```

**Component best practices:**
- Use `#[prop(default)]` for optional props with defaults
- Use `#[prop(optional)]` for truly optional props
- Implement `Children` for components that accept child elements
- Use descriptive prop names and types

### Server Functions

**Creating server functions:**
```rust
#[server]
pub async fn get_user_data(user_id: u32) -> Result<UserData, ServerFnError> {
    use sqlx::PgPool;
    
    let pool = use_context::<PgPool>()
        .expect("Database pool should be provided");
    
    let user = sqlx::query_as!(
        UserData,
        "SELECT * FROM users WHERE id = $1",
        user_id as i32
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(user)
}
```

**Server function guidelines:**
- Always return `Result<T, ServerFnError>`
- Use proper error handling and conversion
- Validate inputs thoroughly
- Consider rate limiting for public endpoints

### Reactive Patterns

**Signal patterns:**
```rust
// Simple state
let (count, set_count) = signal(0);

// Derived state
let double_count = memo(move |_| count.get() * 2);

// Effects for side effects
effect(move |_| {
    log!("Count is now: {}", count.get());
});

// Resources for async data
let user_resource = resource(
    move || user_id.get(),
    |id| async move { fetch_user(id).await }
);
```

**Best practices:**
- Use signals for local component state
- Use contexts for shared state
- Use resources for async data fetching
- Use memos for expensive computations

## Testing

### Unit Testing

**Testing components:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use leptos_dom::view;

    #[test]
    fn test_counter_component() {
        let runtime = create_runtime();
        
        let rendered = view! {
            <Counter initial_value=5/>
        };
        
        // Test initial state, interactions, etc.
        
        runtime.dispose();
    }
}
```

**Testing server functions:**
```rust
#[tokio::test]
async fn test_get_user_data() {
    // Setup test database
    let pool = setup_test_db().await;
    
    // Insert test data
    insert_test_user(&pool, 1, "Test User").await;
    
    // Test server function
    let result = get_user_data(1).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Test User");
}
```

### Integration Testing

**E2E testing with examples:**
```bash
cd examples/todo_app_sqlite_axum/e2e
cargo test
```

**Browser testing:**
```rust
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
async fn test_client_side_functionality() {
    // Test client-side behavior
}
```

### Performance Testing

**Benchmarking:**
```bash
cd benchmarks
cargo bench --bench framework_benchmark
```

**Custom benchmarks:**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_signal_updates(c: &mut Criterion) {
    c.bench_function("signal_update", |b| {
        let runtime = create_runtime();
        let (signal, set_signal) = create_signal(0);
        
        b.iter(|| {
            set_signal.update(|n| *n += 1);
            black_box(signal.get());
        });
        
        runtime.dispose();
    });
}

criterion_group!(benches, bench_signal_updates);
criterion_main!(benches);
```

## Debugging

### Common Issues

**WASM compilation errors:**
```bash
# Check WASM target is installed
rustup target add wasm32-unknown-unknown

# Clear cargo cache
cargo clean

# Rebuild with verbose output
cargo build --target wasm32-unknown-unknown -v
```

**Runtime errors:**
- Check browser console for WASM panics
- Use `web_sys::console::log!` for debugging
- Enable `console_error_panic_hook` for better stack traces

**Hydration mismatches:**
```rust
// Use create_effect to debug hydration
create_effect(move |_| {
    log!("Client state: {:?}, Server state: {:?}", 
         client_value.get(), server_value.get());
});
```

### Debugging Tools

**Logging setup:**
```rust
// In main.rs or lib.rs
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    
    leptos::mount_to_body(App);
}
```

**Performance profiling:**
```rust
use web_sys::console;

let start = js_sys::Date::now();
// ... expensive operation
let end = js_sys::Date::now();
console::log_1(&format!("Operation took {}ms", end - start).into());
```

## Build and Deployment

### Development Build

**Client-side rendering:**
```bash
trunk serve --open
```

**Full-stack development:**
```bash
cargo leptos watch
```

### Production Build

**Optimize for production:**
```bash
# Add to Cargo.toml
[profile.release]
codegen-units = 1
lto = true
opt-level = "z"

# Build optimized
cargo leptos build --release
```

**Bundle analysis:**
```bash
# Install wasm-pack with size optimization
wasm-pack build --release --target web --out-dir pkg

# Analyze bundle size
ls -la pkg/*.wasm
```

### Deployment Strategies

**Static hosting (CSR):**
```bash
trunk build --release
# Deploy dist/ directory to static host
```

**Server hosting (SSR):**
```bash
cargo leptos build --release
# Deploy server binary + static assets
```

**Docker deployment:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo leptos build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/app /usr/local/bin/
COPY --from=builder /app/site /app/site
EXPOSE 3000
CMD ["app"]
```

## Advanced Topics

### Custom Integrations

**Creating server integration:**
```rust
// Example for custom server framework
pub fn leptos_routes<F>(
    options: LeptosOptions,
    routes: Vec<RouteListing>,
    app_fn: F,
) -> impl Service
where
    F: Fn() -> impl IntoView + Clone + Send + 'static,
{
    // Implementation for custom server framework
}
```

### Performance Optimization

**Bundle splitting:**
```rust
// Use lazy imports for large dependencies
let heavy_component = lazy(|| import("./heavy_component"));
```

**Server streaming:**
```rust
// Enable out-of-order streaming
view! {
    <Suspense fallback=|| view! { <div>"Loading..."</div> }>
        <AsyncComponent/>
    </Suspense>
}
```

### Custom Middleware

**Server function middleware:**
```rust
#[server(middleware = [auth_middleware])]
pub async fn protected_function() -> Result<String, ServerFnError> {
    // Protected server function
}

fn auth_middleware() -> impl tower::Layer<Service> {
    // Authentication middleware implementation
}
```

## Resources and Community

### Documentation
- **Book**: https://leptos-rs.github.io/leptos/
- **API Docs**: https://docs.rs/leptos/
- **Examples**: Browse `./examples/` directory

### Community
- **Discord**: https://discord.gg/YdRAhS7eQB
- **GitHub Discussions**: https://github.com/leptos-rs/leptos/discussions
- **Awesome Leptos**: https://github.com/leptos-rs/awesome-leptos

### Getting Help
1. Check existing documentation
2. Search GitHub issues
3. Ask on Discord #help channel
4. Create detailed GitHub issue if needed

---

*This guide covers the essential patterns for Leptos development. For framework development and contributions, see [CONTRIBUTING.md](./CONTRIBUTING.md).*