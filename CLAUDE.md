# Leptos - Full-Stack Rust Web Framework

A full-stack, isomorphic web framework leveraging fine-grained reactivity to build declarative user interfaces in Rust.

## Project Overview

Leptos is a comprehensive web framework that enables building:
- **Single-page applications (SPAs)** with client-side rendering
- **Multi-page applications (MPAs)** with server-side rendering  
- **Progressive web apps** with server-side rendering and hydration

The same Leptos code can be used across all three rendering modes, providing exceptional flexibility and code reuse.

## Core Architecture

### Components
- **leptos** - Core framework and reactive system
- **leptos_dom** - DOM manipulation utilities
- **leptos_router** - Client and server-side routing
- **leptos_server** - Server function integration
- **leptos_macro** - Compile-time macros for views and components
- **reactive_graph** - Fine-grained reactive system
- **tachys** - Rendering engine
- **server_fn** - Isomorphic server functions

### Key Features
- **Fine-grained reactivity** - Updates individual DOM nodes without virtual DOM overhead
- **Isomorphic server functions** - Write server logic alongside client components
- **Web-standard foundation** - Built on standard `<a>` and `<form>` elements
- **Multiple rendering modes** - SSR, CSR, and hydration support
- **Streaming** - Out-of-order and in-order HTML streaming

## Claude Code Integration

### Development Commands
```bash
# Build the framework
cargo build --workspace

# Run tests
cargo test --workspace  

# Run examples
cd examples/counter
cargo leptos watch

# Format code
cargo fmt --all

# Check code quality
cargo clippy --workspace --all-targets --all-features
```

### Project Structure
```
leptos/
├── leptos/           # Main framework crate
├── leptos_dom/       # DOM utilities
├── leptos_router/    # Routing system
├── leptos_server/    # Server functions
├── reactive_graph/   # Reactive system
├── examples/         # Example applications
├── integrations/     # Server integrations (Axum, Actix)
└── benchmarks/       # Performance benchmarks
```

### Testing Infrastructure

The project includes comprehensive testing:
- Unit tests in each crate
- Integration tests for server functions
- End-to-end tests for examples
- Performance benchmarks
- Browser compatibility tests

### Common Development Tasks

**Adding a new feature:**
1. Implement in appropriate crate (`leptos`, `leptos_dom`, etc.)
2. Add unit tests
3. Update documentation
4. Add integration test if needed
5. Update examples if relevant

**Running specific tests:**
```bash
# Test specific crate
cargo test -p leptos

# Test with specific features
cargo test --features "ssr"

# Run browser tests
wasm-pack test --headless --chrome leptos_dom
```

**Working with examples:**
```bash
# List all examples
ls examples/

# Run specific example
cd examples/todo_app_sqlite_axum
cargo leptos watch
```

### Performance Considerations
- Fine-grained reactivity minimizes re-renders
- Server streaming reduces time-to-first-byte
- WASM bundle optimization through tree-shaking
- Efficient hydration with selective activation

### Browser Support
- Modern browsers with WASM support
- Progressive enhancement for older browsers
- Server-side rendering fallback

## Getting Started

### Prerequisites
- Rust 1.75+ (MSRV: 1.88 for workspace)
- `cargo-leptos` for development
- Node.js (for some examples with JS dependencies)

### Quick Start
```bash
# Install cargo-leptos
cargo install cargo-leptos

# Create new project
cargo leptos new --git https://github.com/leptos-rs/start-axum

# Development server
cargo leptos watch
```

### Feature Flags
- `csr` - Client-side rendering
- `ssr` - Server-side rendering  
- `hydrate` - Hydration mode
- `nightly` - Nightly Rust features
- `tracing` - Tracing support

## Resources

- **Website**: https://leptos.dev
- **Book**: https://leptos-rs.github.io/leptos/
- **API Docs**: https://docs.rs/leptos/
- **Examples**: `./examples/`
- **Discord**: https://discord.gg/YdRAhS7eQB
- **Awesome Leptos**: https://github.com/leptos-rs/awesome-leptos

## Development Status

Leptos is actively developed and production-ready:
- Stable APIs with minimal breaking changes
- Growing ecosystem and community
- Used in production applications
- Regular releases and maintenance

## Contributing

See [CONTRIBUTING.md](./docs/CONTRIBUTING.md) for detailed contribution guidelines.

---

*This is a workspace with 40+ crates providing a comprehensive full-stack web development experience in Rust.*