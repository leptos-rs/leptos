# Leptos Init Command Guide

The `leptos init` command is a powerful tool that revolutionizes how you start new Leptos projects. It eliminates the complex manual configuration that previously took 30+ minutes and reduces it to under 1 minute with smart scaffolding.

## 🚀 Quick Start

```bash
# Install the leptos-init tool
cargo install leptos-init

# Create a new project (interactive mode)
leptos init my-awesome-app

# Create with specific template
leptos init my-spa --template spa
leptos init my-api --template api --database sqlite
```

## 📋 Available Templates

### 1. **Fullstack** (Default - Recommended)
Perfect for most applications with server-side rendering and client hydration.

```bash
leptos init my-app --template fullstack
```

**Features:**
- Server-side rendering (SSR)
- Client-side hydration
- Axum server backend
- Optional database integration
- Complete project structure

### 2. **SPA** (Single Page Application)
Client-side only applications with no server component.

```bash
leptos init my-spa --template spa
```

**Features:**
- Client-side rendering (CSR)
- No server dependencies
- Optimized for static hosting
- Fast initial load

### 3. **API** (Server Functions Only)
Backend API with server functions, no frontend.

```bash
leptos init my-api --template api
```

**Features:**
- Server functions only
- RESTful API endpoints
- Database integration ready
- No client-side code

### 4. **Static** (Static Site Generation)
Pre-rendered static sites with minimal JavaScript.

```bash
leptos init my-blog --template static
```

**Features:**
- Static site generation
- SEO optimized
- Fast loading
- CDN friendly

### 5. **Custom** (Interactive Wizard)
Guided setup for complex requirements.

```bash
leptos init my-custom --template custom
```

## ⚙️ Configuration Options

### Server Backends
Choose your server framework:

```bash
leptos init my-app --server axum    # Default, recommended
leptos init my-app --server actix   # Actix Web
leptos init my-app --server warp    # Warp
```

### Database Integration
Add database support:

```bash
leptos init my-app --database sqlite      # SQLite (recommended for development)
leptos init my-app --database postgresql  # PostgreSQL
leptos init my-app --database mysql       # MySQL
```

### Styling Frameworks
Include styling solutions:

```bash
leptos init my-app --styling tailwind     # Tailwind CSS
leptos init my-app --styling vanilla-css  # Vanilla CSS
leptos init my-app --styling sass         # Sass/SCSS
```

### Advanced Features
Enable additional capabilities:

```bash
leptos init my-app --tracing    # Enable tracing/logging
leptos init my-app --islands    # Enable islands architecture
```

## 🛠️ Command Reference

### Basic Usage
```bash
leptos init <NAME> [OPTIONS]
```

### Arguments
- `<NAME>` - Project name (required)
  - Must start with a letter
  - Can contain lowercase letters, numbers, underscores, and hyphens
  - Examples: `my-app`, `blog_site`, `api-v2`

### Options
- `--template <TEMPLATE>` - Project template (default: fullstack)
- `--server <SERVER>` - Server backend (default: axum)
- `--database <DATABASE>` - Database integration (default: none)
- `--styling <STYLING>` - Styling framework (default: none)
- `--tracing` - Enable tracing/logging
- `--islands` - Enable islands architecture
- `--force` - Overwrite existing directory
- `--target <TARGET>` - Target directory (default: current directory)
- `--interactive` - Run in interactive mode
- `--help` - Show help information

## 📁 Generated Project Structure

### Fullstack Template
```
my-app/
├── Cargo.toml              # Project configuration
├── build.rs                # Build script with validation
├── src/
│   ├── main.rs             # Server and client entry points
│   ├── app.rs              # Main application component
│   └── validation_examples.rs  # Compile-time validation examples
├── public/
│   └── index.html          # HTML template
├── README.md               # Project documentation
└── .gitignore              # Git ignore rules
```

### SPA Template
```
my-spa/
├── Cargo.toml              # Client-only configuration
├── src/
│   ├── main.rs             # Client entry point
│   └── app.rs              # Application component
├── public/
│   └── index.html          # HTML template
└── README.md               # Project documentation
```

### API Template
```
my-api/
├── Cargo.toml              # Server-only configuration
├── src/
│   └── main.rs             # Server entry point
└── README.md               # API documentation
```

## 🎯 Smart Defaults

The `leptos init` command provides intelligent defaults based on your template choice:

| Template | Server | Database | Styling | Features |
|----------|--------|----------|---------|----------|
| Fullstack | Axum | SQLite | None | SSR + Hydrate |
| SPA | None | None | None | CSR only |
| API | Axum | None | None | SSR only |
| Static | None | None | None | SSR only |
| Custom | Configurable | Configurable | Configurable | Configurable |

## 🔧 Generated Configuration

### Cargo.toml Features
The generated `Cargo.toml` includes:

- **Smart Dependencies**: Only includes what you need
- **Feature Flags**: Properly configured for your template
- **Leptos Metadata**: Complete configuration for cargo-leptos
- **Build Dependencies**: Compile-time validation support

### Example Generated Cargo.toml (Fullstack)
```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.8", features = ["tracing"] }
leptos_axum = { version = "0.8", optional = true }
leptos_router = { version = "0.8" }
axum = { version = "0.8", optional = true }
# ... other dependencies

[features]
default = []
hydrate = ["leptos/hydrate"]
ssr = ["leptos/ssr", "dep:leptos_axum", "dep:axum", "dep:tokio"]

[package.metadata.leptos]
output-name = "my-app"
site-addr = "127.0.0.1:3000"
bin-features = ["ssr"]
lib-features = ["hydrate"]
# ... other metadata
```

## 🚀 Next Steps After Creation

1. **Navigate to your project**:
   ```bash
   cd my-app
   ```

2. **Start development server**:
   ```bash
   cargo leptos watch
   ```

3. **Open in browser**:
   ```
   http://127.0.0.1:3000
   ```

4. **Build for production**:
   ```bash
   cargo leptos build --release
   ```

## 🎨 Customization Examples

### E-commerce Site
```bash
leptos init ecommerce --template fullstack --database postgresql --styling tailwind --tracing
```

### Personal Blog
```bash
leptos init blog --template static --styling tailwind
```

### API Service
```bash
leptos init api-service --template api --database sqlite --server axum
```

### Admin Dashboard
```bash
leptos init admin --template spa --styling tailwind --islands
```

## 🔍 Troubleshooting

### Common Issues

**Project name validation error**:
```
❌ Error: Invalid project name '123invalid'
```
- Project names must start with a letter
- Use only lowercase letters, numbers, underscores, and hyphens

**Directory already exists**:
```
❌ Error: Directory 'my-app' already exists
```
- Use `--force` to overwrite: `leptos init my-app --force`
- Or choose a different name

**Missing dependencies**:
```
error: could not find `Cargo.toml`
```
- Make sure you're running from the correct directory
- Use `--target` to specify the target directory

### Getting Help

- **Command help**: `leptos init --help`
- **Documentation**: https://leptos.dev
- **Book**: https://leptos-rs.github.io/leptos/
- **Discord**: https://discord.gg/YdRAhS7eQB

## 🎉 Benefits

### Before leptos init
- ❌ 30+ minutes manual setup
- ❌ Complex Cargo.toml configuration
- ❌ Feature flag confusion
- ❌ Dependency management errors
- ❌ Missing build scripts
- ❌ No validation system

### After leptos init
- ✅ Under 1 minute setup
- ✅ Smart configuration
- ✅ Proper feature flags
- ✅ Correct dependencies
- ✅ Build validation
- ✅ Ready to code

## 🔮 Future Enhancements

The `leptos init` command is actively developed with planned features:

- **Template Marketplace**: Community-contributed templates
- **Plugin System**: Extensible template system
- **Migration Tools**: Upgrade existing projects
- **IDE Integration**: VS Code and other editor support
- **Cloud Templates**: Deploy-ready configurations

---

**Happy coding with Leptos!** 🚀

For more information, visit [leptos.dev](https://leptos.dev) or join our [Discord community](https://discord.gg/YdRAhS7eQB).
