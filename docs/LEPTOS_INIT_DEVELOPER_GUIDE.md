# Leptos Init Developer Guide

This guide is for developers contributing to the `leptos init` command and template system.

## 🏗️ Architecture Overview

The `leptos init` system consists of several key components:

```
leptos_init/
├── src/
│   ├── lib.rs              # Core library with templates and generators
│   ├── cli.rs              # Interactive CLI wizard
│   └── bin/
│       └── leptos-init.rs  # Command-line interface
├── tests/
│   └── cli_tests.rs        # TDD test suite
└── Cargo.toml              # Package configuration
```

## 🧩 Core Components

### 1. InitConfig
Central configuration structure that defines project settings:

```rust
pub struct InitConfig {
    pub name: String,
    pub template: ProjectTemplate,
    pub server: ServerBackend,
    pub database: Database,
    pub styling: Styling,
    pub features: Vec<String>,
    pub use_tracing: bool,
    pub use_islands: bool,
}
```

### 2. ProjectTemplate
Enum defining available project types:

```rust
pub enum ProjectTemplate {
    Spa,        // Client-side only
    Fullstack,  // SSR + Client hydration
    Static,     // Static site generation
    Api,        // Server functions only
    Custom,     // Interactive wizard
}
```

### 3. ProjectGenerator
Main class responsible for generating complete projects:

```rust
pub struct ProjectGenerator {
    config: InitConfig,
    target_path: PathBuf,
}
```

## 🔧 Adding New Templates

### Step 1: Define Template
Add your template to the `ProjectTemplate` enum:

```rust
pub enum ProjectTemplate {
    // ... existing templates
    Ecommerce,  // New template
}
```

### Step 2: Configure Defaults
Update `InitConfig::for_template()`:

```rust
ProjectTemplate::Ecommerce => {
    config.features = vec!["ssr".to_string(), "hydrate".to_string()];
    config.database = Database::Postgresql;
    config.styling = Styling::Tailwind;
    config.use_tracing = true;
}
```

### Step 3: Generate Dependencies
Update `InitConfig::dependencies()`:

```rust
// Add ecommerce-specific dependencies
if self.template == ProjectTemplate::Ecommerce {
    deps.insert("stripe".to_string(), r#"{ version = "0.28" }"#.to_string());
    deps.insert("uuid".to_string(), r#"{ version = "1.0", features = ["v4"] }"#.to_string());
}
```

### Step 4: Configure Features
Update `InitConfig::feature_flags()`:

```rust
ProjectTemplate::Ecommerce => {
    features.insert("default".to_string(), vec![]);
    features.insert("hydrate".to_string(), vec!["leptos/hydrate".to_string()]);
    
    let mut ssr_features = vec!["leptos/ssr".to_string()];
    ssr_features.extend(vec![
        "dep:leptos_axum".to_string(),
        "dep:axum".to_string(),
        "dep:tokio".to_string(),
        "dep:stripe".to_string(),
    ]);
    features.insert("ssr".to_string(), ssr_features);
}
```

### Step 5: Generate Source Files
Add template-specific source generation:

```rust
fn generate_ecommerce_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
    // Generate ecommerce-specific main.rs
    let main_rs = r#"
use leptos::prelude::*;
use leptos_axum::*;

mod app;
mod components;
mod pages;
mod services;

#[tokio::main]
async fn main() {
    // Ecommerce-specific setup
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(app::App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .nest_service("/pkg", ServeDir::new("target/site/pkg"))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Ecommerce server listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
"#;
    
    std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;
    
    // Generate additional files
    self.generate_ecommerce_components()?;
    self.generate_ecommerce_pages()?;
    self.generate_ecommerce_services()?;
    
    Ok(())
}
```

### Step 6: Update CLI
Add template to clap enum:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
pub enum ProjectTemplate {
    // ... existing templates
    Ecommerce,
}
```

### Step 7: Add Tests
Create tests for your new template:

```rust
#[test]
fn test_leptos_init_ecommerce_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("ecommerce-app");
    
    let output = run_leptos_init(&["ecommerce-app", "--template", "ecommerce"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init ecommerce should succeed");
    
    // Verify ecommerce-specific structure
    assert!(project_path.join("src/components").exists(), "Should have components directory");
    assert!(project_path.join("src/pages").exists(), "Should have pages directory");
    assert!(project_path.join("src/services").exists(), "Should have services directory");
    
    // Verify dependencies
    let cargo_toml = std::fs::read_to_string(project_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("stripe"), "Should include stripe dependency");
    assert!(cargo_toml.contains("uuid"), "Should include uuid dependency");
}
```

## 🧪 Testing Guidelines

### TDD Approach
We use Test-Driven Development:

1. **Red**: Write failing tests first
2. **Green**: Implement functionality to make tests pass
3. **Refactor**: Improve code while keeping tests green

### Test Structure
```rust
/// Test that leptos init creates [template] template correctly
#[test]
fn test_leptos_init_[template]_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path().join("[template]-app");
    
    let output = run_leptos_init(&["[template]-app", "--template", "[template]"], Some(temp_dir.path()));

    assert!(output.status.success(), "leptos-init [template] should succeed");
    
    // Verify template-specific structure
    // ... assertions
}
```

### Running Tests
```bash
# Run all tests
cargo test --test cli_tests

# Run specific test
cargo test --test cli_tests test_leptos_init_ecommerce_template

# Run with output
cargo test --test cli_tests -- --nocapture
```

## 🔧 Adding New Server Backends

### Step 1: Define Backend
Add to `ServerBackend` enum:

```rust
pub enum ServerBackend {
    // ... existing backends
    Warp,  // New backend
}
```

### Step 2: Configure Dependencies
Update `InitConfig::dependencies()`:

```rust
ServerBackend::Warp => {
    deps.insert("leptos_warp".to_string(), r#"{ version = "0.8", optional = true }"#.to_string());
    deps.insert("warp".to_string(), r#"{ version = "0.3", optional = true }"#.to_string());
}
```

### Step 3: Configure Features
Update `InitConfig::feature_flags()`:

```rust
ServerBackend::Warp => {
    ssr_features.extend(vec![
        "dep:leptos_warp".to_string(),
        "dep:warp".to_string(),
    ]);
}
```

### Step 4: Generate Source
Update source generation for server backends:

```rust
fn generate_warp_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
    let main_rs = r#"
use warp::Filter;
use leptos::prelude::*;

#[tokio::main]
async fn main() {
    // Warp-specific setup
    let routes = warp::path("api")
        .and(warp::get())
        .map(|| "Hello from Warp!");

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3000))
        .await;
}
"#;
    
    std::fs::write(self.target_path.join("src/main.rs"), main_rs)?;
    Ok(())
}
```

## 🎨 Adding New Styling Frameworks

### Step 1: Define Framework
Add to `Styling` enum:

```rust
pub enum Styling {
    // ... existing frameworks
    Bootstrap,  // New framework
}
```

### Step 2: Configure Assets
Update asset generation:

```rust
if self.config.styling == Styling::Bootstrap {
    let bootstrap_css = r#"
/* Bootstrap CSS */
@import url('https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css');

.custom-navbar {
    background-color: #007bff;
}
"#;
    std::fs::write(self.target_path.join("src/styles/bootstrap.css"), bootstrap_css)?;
}
```

### Step 3: Update Metadata
Update `InitConfig::leptos_metadata()`:

```rust
if self.styling == Styling::Bootstrap {
    metadata.insert("style-file".to_string(), "src/styles/bootstrap.css".to_string());
}
```

## 🔍 Debugging

### Common Issues

**Template not generating correctly**:
- Check `InitConfig::for_template()` for correct defaults
- Verify `ProjectGenerator::generate_source_files()` calls your template method
- Ensure all required directories are created

**Dependencies missing**:
- Verify `InitConfig::dependencies()` includes all required crates
- Check feature flags in `InitConfig::feature_flags()`
- Ensure optional dependencies are properly marked

**Tests failing**:
- Use `--nocapture` to see output
- Check that `run_leptos_init()` helper is used correctly
- Verify test assertions match actual generated content

### Debug Commands
```bash
# Build with debug info
cargo build --bin leptos-init

# Run with verbose output
RUST_LOG=debug cargo run --bin leptos-init -- my-app --template fullstack

# Test specific functionality
cargo test --test cli_tests test_leptos_init_fullstack_template -- --nocapture
```

## 📚 Code Style

### Rust Conventions
- Use `snake_case` for functions and variables
- Use `PascalCase` for types and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefer `Result<T, E>` over panicking
- Use `?` operator for error propagation

### Documentation
- Document all public functions with `///`
- Include examples in documentation
- Use `# Examples` sections for complex functions
- Document error conditions

### Error Handling
```rust
// Good: Use Result types
fn generate_file(&self) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(self.path, content)?;
    Ok(())
}

// Bad: Panic on errors
fn generate_file(&self) {
    std::fs::write(self.path, content).unwrap();
}
```

## 🚀 Contributing

### Pull Request Process
1. Fork the repository
2. Create a feature branch
3. Write tests first (TDD)
4. Implement functionality
5. Ensure all tests pass
6. Update documentation
7. Submit pull request

### Code Review Checklist
- [ ] Tests are comprehensive and pass
- [ ] Code follows Rust conventions
- [ ] Documentation is updated
- [ ] No breaking changes (unless intentional)
- [ ] Error handling is proper
- [ ] Performance is acceptable

## 🔮 Future Enhancements

### Planned Features
- **Template Marketplace**: Community templates
- **Plugin System**: Extensible architecture
- **Migration Tools**: Upgrade existing projects
- **IDE Integration**: Editor support
- **Cloud Templates**: Deployment-ready configs

### Architecture Improvements
- **Async Generation**: Parallel file creation
- **Template Caching**: Faster subsequent runs
- **Validation System**: Enhanced error checking
- **Configuration UI**: Web-based setup

---

**Happy contributing!** 🚀

For questions or help, join our [Discord community](https://discord.gg/YdRAhS7eQB) or open an issue on GitHub.
