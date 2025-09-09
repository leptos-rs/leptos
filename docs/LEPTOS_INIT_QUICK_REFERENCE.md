# Leptos Init Quick Reference

## 🚀 Basic Commands

```bash
# Install
cargo install leptos-init

# Create project (interactive)
leptos init my-app

# Create with template
leptos init my-app --template spa
leptos init my-app --template fullstack
leptos init my-app --template api
leptos init my-app --template static
```

## 📋 Templates

| Template | Description | Server | Client | Use Case |
|----------|-------------|--------|--------|----------|
| `fullstack` | SSR + Hydration | ✅ Axum | ✅ Hydrate | Web apps |
| `spa` | Client-only | ❌ | ✅ CSR | Static sites |
| `api` | Server-only | ✅ Axum | ❌ | Backend APIs |
| `static` | Static generation | ❌ | ✅ SSR | Blogs, docs |
| `custom` | Interactive wizard | Configurable | Configurable | Complex needs |

## ⚙️ Options

```bash
# Server backends
--server axum      # Default, recommended
--server actix     # Actix Web
--server warp      # Warp

# Databases
--database sqlite      # SQLite (dev)
--database postgresql  # PostgreSQL
--database mysql       # MySQL

# Styling
--styling tailwind     # Tailwind CSS
--styling vanilla-css  # Vanilla CSS
--styling sass         # Sass/SCSS

# Features
--tracing    # Enable logging
--islands    # Islands architecture
--force      # Overwrite existing
--interactive # Guided setup
```

## 🎯 Common Patterns

### E-commerce Site
```bash
leptos init shop --template fullstack --database postgresql --styling tailwind --tracing
```

### Personal Blog
```bash
leptos init blog --template static --styling tailwind
```

### API Service
```bash
leptos init api --template api --database sqlite --server axum
```

### Admin Dashboard
```bash
leptos init admin --template spa --styling tailwind --islands
```

### Mobile App Backend
```bash
leptos init mobile-api --template api --database postgresql --server axum --tracing
```

## 📁 Generated Structure

### Fullstack
```
my-app/
├── Cargo.toml          # Dependencies & features
├── build.rs            # Build validation
├── src/
│   ├── main.rs         # Server + client entry
│   ├── app.rs          # Main component
│   └── validation_examples.rs
├── public/
│   └── index.html      # HTML template
└── README.md           # Documentation
```

### SPA
```
my-spa/
├── Cargo.toml          # Client-only deps
├── src/
│   ├── main.rs         # Client entry
│   └── app.rs          # App component
├── public/
│   └── index.html      # HTML template
└── README.md           # Documentation
```

### API
```
my-api/
├── Cargo.toml          # Server-only deps
├── src/
│   └── main.rs         # Server entry
└── README.md           # API docs
```

## 🔧 Next Steps

```bash
# 1. Navigate to project
cd my-app

# 2. Start development
cargo leptos watch

# 3. Open browser
open http://127.0.0.1:3000

# 4. Build for production
cargo leptos build --release
```

## 🐛 Troubleshooting

### Project Name Issues
```bash
# ❌ Invalid names
leptos init 123invalid    # Starts with number
leptos init My-App        # Uppercase letters

# ✅ Valid names
leptos init my-app        # Lowercase + hyphens
leptos init blog_site     # Underscores OK
leptos init api-v2        # Numbers OK after letter
```

### Directory Conflicts
```bash
# Directory exists
leptos init my-app        # ❌ Fails if exists
leptos init my-app --force # ✅ Overwrites
```

### Missing Dependencies
```bash
# Wrong directory
cd /some/empty/dir
leptos init my-app        # ❌ No Cargo.toml

# Correct approach
cd /my/projects
leptos init my-app        # ✅ Works
```

## 📚 Help & Resources

```bash
# Command help
leptos init --help

# Online resources
https://leptos.dev                    # Main site
https://leptos-rs.github.io/leptos/   # Book
https://discord.gg/YdRAhS7eQB         # Discord
```

## 🎨 Customization Examples

### Full E-commerce Setup
```bash
leptos init ecommerce \
  --template fullstack \
  --server axum \
  --database postgresql \
  --styling tailwind \
  --tracing \
  --islands
```

### Minimal API
```bash
leptos init minimal-api \
  --template api \
  --server axum
```

### Static Blog with Styling
```bash
leptos init blog \
  --template static \
  --styling tailwind
```

### SPA with Islands
```bash
leptos init dashboard \
  --template spa \
  --styling tailwind \
  --islands
```

## 🔍 Validation Rules

### Project Names
- ✅ Start with letter
- ✅ Lowercase letters only
- ✅ Numbers, underscores, hyphens OK
- ❌ No uppercase
- ❌ No special characters
- ❌ Can't start with number

### Examples
```bash
# Valid
my-app
blog_site
api-v2
project123
test_app_v1

# Invalid
123invalid
My-App
project@site
my.app
```

## 🚀 Performance Tips

### Fast Development
```bash
# Use SPA for rapid prototyping
leptos init prototype --template spa

# Use SQLite for development
leptos init dev-app --database sqlite

# Skip styling initially
leptos init quick-start --template fullstack
```

### Production Ready
```bash
# Use PostgreSQL for production
leptos init prod-app --database postgresql

# Enable tracing for monitoring
leptos init prod-app --tracing

# Use fullstack for SEO
leptos init prod-app --template fullstack
```

---

**Quick Start**: `leptos init my-app` → `cd my-app` → `cargo leptos watch` → `open http://127.0.0.1:3000` 🚀
