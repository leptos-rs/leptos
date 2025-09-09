# Automatic Mode Detection Example

This example demonstrates the new automatic mode detection system that eliminates the need for manual feature flag configuration in Leptos projects.

## Features Demonstrated

- **Automatic Mode Detection**: The system automatically detects that this is a fullstack application based on the code patterns
- **Context-Aware Validation**: Compile-time validation ensures server-only code isn't used in client context and vice versa
- **Server Functions**: Automatic handling of client/server context with server functions
- **Compile-Time Safety**: Build-time validation prevents configuration errors

## Key Components

### 1. Mode Configuration

Instead of manual feature flags, we use a simple mode declaration:

```toml
[package.metadata.leptos]
mode = "fullstack"
env = "DEV"
```

### 2. Context-Aware Types

```rust
// Server-only type - compile error if used in client
#[derive(ContextAware)]
#[leptos::server_only]
pub struct DatabaseConnection { ... }

// Client-only type - compile error if used in server
#[derive(ContextAware)]
#[leptos::client_only]
pub struct WebSocketClient { ... }

// Universal type - can be used anywhere
#[derive(ContextAware)]
#[leptos::universal]
pub struct User { ... }
```

### 3. Context-Aware Code Blocks

```rust
// Server-only code block
server_only! {
    let db = DatabaseConnection::new();
    // This code only compiles in server context
}

// Client-only code block
client_only! {
    let ws = WebSocketClient::new();
    // This code only compiles in client context
}
```

### 4. Automatic Validation

The build system automatically validates:
- Feature flag conflicts
- Context mismatches
- Invalid mode configurations
- Missing required features

## Running the Example

### Prerequisites

- Rust 1.70+
- Node.js (for the frontend build)

### Development Server

```bash
# Start the development server
cargo leptos watch

# Or manually:
cargo run
```

### Production Build

```bash
# Build for production
cargo leptos build --release

# Serve the built files
cargo leptos serve
```

## What This Example Shows

1. **No Manual Feature Flags**: The project doesn't need to manually configure `ssr`, `hydrate`, or `csr` features
2. **Automatic Detection**: The system detects this is a fullstack app based on:
   - Presence of server functions
   - Client-side components
   - Both `main.rs` and `lib.rs` files
3. **Compile-Time Safety**: Attempting to use server-only code in client context (or vice versa) results in compile errors
4. **Simplified Configuration**: Just declare the mode and let the system handle the rest

## Migration from Manual Features

If you're migrating from manual feature flags, you can use the CLI tool:

```bash
# Analyze your current project
leptos-mode analyze

# Migrate to automatic mode detection
leptos-mode migrate
```

## Benefits

- **Eliminates Configuration Errors**: No more conflicting feature flags
- **Simplifies Development**: Just declare your mode and start coding
- **Compile-Time Safety**: Catch context errors at build time
- **Better Developer Experience**: Clear error messages and suggestions
- **Automatic Optimization**: The system optimizes features based on your mode

## Troubleshooting

### Common Issues

1. **"Type can only be used in server context"**
   - Solution: Move the code to a server function or use client-only alternatives

2. **"Feature conflict detected"**
   - Solution: Use the CLI tool to migrate: `leptos-mode migrate`

3. **"Invalid mode for target"**
   - Solution: Check that your mode matches your build target

### Getting Help

- Run `leptos-mode help <mode>` for mode-specific help
- Check the [Leptos documentation](https://leptos.dev) for detailed guides
- Use `leptos-mode analyze` to diagnose configuration issues
