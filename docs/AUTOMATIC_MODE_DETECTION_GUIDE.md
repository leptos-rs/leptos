# Automatic Mode Detection Guide

The automatic mode detection system eliminates the need for manual feature flag configuration in Leptos projects. Instead of manually managing `ssr`, `hydrate`, `csr`, and other features, you simply declare your application mode and let the system handle the rest.

## Table of Contents

- [Overview](#overview)
- [Supported Modes](#supported-modes)
- [Getting Started](#getting-started)
- [Migration Guide](#migration-guide)
- [Context-Aware Development](#context-aware-development)
- [CLI Tool](#cli-tool)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)

## Overview

### The Problem with Manual Feature Flags

Traditional Leptos projects require manual configuration of feature flags:

```toml
# Old way - manual feature management
[features]
default = ["ssr", "hydrate"]
csr = []
ssr = []
hydrate = []

[package.metadata.leptos]
bin-features = ["ssr"]
lib-features = ["hydrate"]
```

This approach has several problems:
- **Configuration Errors**: Easy to create conflicting feature combinations
- **Build Complexity**: Different features for different targets
- **Developer Confusion**: Unclear which features to use when
- **Maintenance Burden**: Manual updates when changing project structure

### The Solution: Automatic Mode Detection

The new system uses intelligent mode detection:

```toml
# New way - automatic mode detection
[package.metadata.leptos]
mode = "fullstack"
env = "DEV"
```

Benefits:
- **Zero Configuration**: Just declare your mode
- **Automatic Optimization**: Features are optimized based on your mode
- **Compile-Time Safety**: Catch configuration errors at build time
- **Better DX**: Clear error messages and suggestions

## Supported Modes

### SPA (Single Page Application)
- **Use case**: Client-side only applications
- **Features**: CSR (Client-Side Rendering)
- **Best for**: Static sites, prototypes, client-only apps

```toml
[package.metadata.leptos]
mode = "spa"
```

### Fullstack
- **Use case**: Full-stack web applications
- **Features**: SSR + Hydration
- **Best for**: Production apps, SEO-important sites

```toml
[package.metadata.leptos]
mode = "fullstack"
```

### Static
- **Use case**: Static site generation
- **Features**: Pre-rendered HTML
- **Best for**: Documentation, blogs, marketing pages

```toml
[package.metadata.leptos]
mode = "static"
```

### API
- **Use case**: Server-only applications
- **Features**: Server-side rendering, API endpoints
- **Best for**: REST APIs, backend services

```toml
[package.metadata.leptos]
mode = "api"
```

## Getting Started

### 1. Install the CLI Tool

```bash
cargo install --path leptos_mode_cli
```

### 2. Analyze Your Project

```bash
leptos-mode analyze
```

This will:
- Detect your current project structure
- Identify the appropriate mode
- Find configuration issues
- Provide migration recommendations

### 3. Migrate Your Project

```bash
leptos-mode migrate
```

This will:
- Update your `Cargo.toml` with mode declarations
- Remove manual feature flag configurations
- Setup automatic validation
- Create backups of original files

### 4. Add Validation Dependencies

Add these to your `Cargo.toml`:

```toml
[dependencies]
leptos_compile_validator = { path = "../leptos_compile_validator" }
leptos_compile_validator_derive = { path = "../leptos_compile_validator_derive" }

[build-dependencies]
leptos_compile_validator = { path = "../leptos_compile_validator", features = ["build"] }
```

### 5. Create build.rs

```rust
use leptos_compile_validator::validate_with_context;

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTOS_MODE");
    println!("cargo:rerun-if-env-changed=LEPTOS_TARGET");
    
    let validation_result = validate_with_context();
    if !validation_result.is_empty() {
        panic!("Leptos validation failed");
    }
}
```

## Migration Guide

### Step 1: Analyze Current Configuration

```bash
leptos-mode analyze --verbose
```

Example output:
```
📊 Analysis Results
==================================================
Detected Mode: Fullstack (confidence: 85.0%)

Current Features:
  • ssr
  • hydrate

⚠️  Issues Found:
  ❌ Conflicting feature flags detected in conditional compilation

💡 Recommendations:
  1. Add mode declaration
     Replace manual feature flags with automatic mode detection
  2. Remove conflicting features
     Multiple rendering mode features can cause build issues
```

### Step 2: Backup Your Project

```bash
leptos-mode migrate --backup
```

This creates a `.leptos-backup/` directory with your original files.

### Step 3: Apply Migration

The migration process will:

1. **Update Cargo.toml**:
   ```toml
   # Before
   [features]
   default = ["ssr", "hydrate"]
   
   [package.metadata.leptos]
   bin-features = ["ssr"]
   lib-features = ["hydrate"]
   
   # After
   [package.metadata.leptos]
   mode = "fullstack"
   env = "DEV"
   ```

2. **Add Validation Dependencies**:
   ```toml
   [dependencies]
   leptos_compile_validator = { path = "../leptos_compile_validator" }
   leptos_compile_validator_derive = { path = "../leptos_compile_validator_derive" }
   ```

3. **Create build.rs**:
   ```rust
   use leptos_compile_validator::validate_with_context;
   
   fn main() {
       let validation_result = validate_with_context();
       if !validation_result.is_empty() {
           panic!("Leptos validation failed");
       }
   }
   ```

### Step 4: Update Your Code

Replace manual feature flag checks with context-aware annotations:

```rust
// Before - manual feature flags
#[cfg(feature = "ssr")]
fn server_only_function() {
    // server code
}

#[cfg(not(feature = "ssr"))]
fn client_only_function() {
    // client code
}

// After - context-aware annotations
use leptos_compile_validator_derive::*;

#[server_only_fn]
fn server_only_function() {
    // server code
}

#[client_only_fn]
fn client_only_function() {
    // client code
}
```

### Step 5: Verify Migration

```bash
cargo check
```

The build system will validate your configuration and catch any issues.

## Context-Aware Development

### Context-Aware Types

Use derive macros to create context-aware types:

```rust
use leptos_compile_validator_derive::*;

// Server-only type
#[derive(ContextAware)]
#[leptos::server_only]
pub struct DatabaseConnection {
    pub connection_string: String,
}

// Client-only type
#[derive(ContextAware)]
#[leptos::client_only]
pub struct WebSocketClient {
    pub url: String,
}

// Universal type
#[derive(ContextAware)]
#[leptos::universal]
pub struct User {
    pub id: u32,
    pub name: String,
}
```

### Context-Aware Code Blocks

Use macros for context-aware code blocks:

```rust
use leptos_compile_validator_derive::*;

// Server-only code block
server_only! {
    let db = DatabaseConnection::new();
    let result = db.query("SELECT * FROM users").await?;
}

// Client-only code block
client_only! {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
}
```

### Context-Aware Functions

Use attribute macros for context-aware functions:

```rust
use leptos_compile_validator_derive::*;

#[server_only_fn]
async fn database_query() -> Result<String, ServerFnError> {
    // This function can only be called from server context
    let db = DatabaseConnection::new();
    Ok(db.query("SELECT * FROM users").await?)
}

#[client_only_fn]
fn setup_web_socket() -> Result<(), JsValue> {
    // This function can only be called from client context
    let ws = WebSocket::new("ws://localhost:8080")?;
    Ok(())
}
```

## CLI Tool

### Commands

#### Analyze
```bash
# Basic analysis
leptos-mode analyze

# Verbose output
leptos-mode analyze --verbose

# JSON output
leptos-mode analyze --format json

# Specific directory
leptos-mode analyze --path /path/to/project
```

#### Migrate
```bash
# Interactive migration
leptos-mode migrate

# Force migration
leptos-mode migrate --force

# No backup
leptos-mode migrate --no-backup
```

#### Validate
```bash
# Check configuration
leptos-mode validate

# Fix issues automatically
leptos-mode validate --fix
```

#### Generate
```bash
# Generate SPA configuration
leptos-mode generate --mode spa

# Generate fullstack configuration for production
leptos-mode generate --mode fullstack --env production

# Save to file
leptos-mode generate --mode static --output leptos.toml
```

#### Help
```bash
# Get help for specific mode
leptos-mode help spa
leptos-mode help fullstack
```

## Troubleshooting

### Common Issues

#### 1. "Conflicting features detected"

**Problem**: Multiple rendering mode features are enabled simultaneously.

**Solution**:
```bash
leptos-mode migrate
```

#### 2. "Type can only be used in server context"

**Problem**: Trying to use server-only code in client context.

**Solution**: Move the code to a server function or use client-only alternatives:

```rust
// Wrong - using server code in client
#[component]
fn MyComponent() -> impl IntoView {
    let db = DatabaseConnection::new(); // ❌ Compile error
    view! { <div></div> }
}

// Right - use server function
#[component]
fn MyComponent() -> impl IntoView {
    let data = create_resource(|| (), |_| get_data_from_server());
    view! { <div></div> }
}

#[server]
async fn get_data_from_server() -> Result<String, ServerFnError> {
    let db = DatabaseConnection::new(); // ✅ OK in server context
    Ok("data".to_string())
}
```

#### 3. "Invalid mode for target"

**Problem**: The declared mode doesn't match your build target.

**Solution**: Check your mode declaration and build target:

```bash
# Check current mode
leptos-mode analyze

# Generate correct configuration
leptos-mode generate --mode <correct-mode>
```

#### 4. "Missing required features"

**Problem**: Required features for your mode are not enabled.

**Solution**: The system will automatically suggest the correct features:

```bash
leptos-mode validate --fix
```

### Getting Help

1. **Use the CLI tool**:
   ```bash
   leptos-mode analyze
   leptos-mode help <mode>
   ```

2. **Check error messages**: The system provides detailed error messages with suggestions.

3. **Review documentation**: Check the [Leptos documentation](https://leptos.dev) for detailed guides.

4. **Community support**: Join the [Leptos Discord](https://discord.gg/leptos) for help.

## Advanced Usage

### Custom Modes

For advanced use cases, you can define custom modes:

```rust
use leptos_mode_resolver::{BuildMode, ModeResolver};

let custom_mode = BuildMode::Custom {
    client_features: vec!["csr".to_string(), "custom-feature".to_string()],
    server_features: vec!["ssr".to_string()],
};

let resolver = ModeResolver::new(custom_mode);
```

### Environment-Specific Configuration

```toml
[package.metadata.leptos]
mode = "fullstack"
env = "DEV"  # or "PROD" or "TEST"
```

### Build Script Integration

```rust
// build.rs
use leptos_compile_validator::validate_with_context;

fn main() {
    // Set environment variables for validation
    std::env::set_var("LEPTOS_MODE", "fullstack");
    std::env::set_var("LEPTOS_TARGET", "server");
    
    let validation_result = validate_with_context();
    if !validation_result.is_empty() {
        panic!("Leptos validation failed");
    }
}
```

### CI/CD Integration

```yaml
# .github/workflows/ci.yml
- name: Validate Leptos Configuration
  run: |
    cargo install --path leptos_mode_cli
    leptos-mode validate
```

## Best Practices

1. **Start with analysis**: Always run `leptos-mode analyze` before making changes.

2. **Use context-aware annotations**: Prefer `#[server_only_fn]` over manual `#[cfg]` attributes.

3. **Validate early**: Add validation to your build process to catch issues early.

4. **Keep it simple**: Let the system handle feature management - avoid manual overrides.

5. **Test thoroughly**: After migration, test both client and server builds.

6. **Use the CLI**: The CLI tool is your friend - use it for analysis, migration, and validation.

## Conclusion

The automatic mode detection system simplifies Leptos development by eliminating manual feature flag management. By declaring your application mode and using context-aware annotations, you get:

- **Simplified Configuration**: No more manual feature flags
- **Compile-Time Safety**: Catch errors at build time
- **Better Developer Experience**: Clear error messages and suggestions
- **Automatic Optimization**: Features are optimized based on your mode

Start your migration today with `leptos-mode analyze` and experience the benefits of automatic mode detection!
