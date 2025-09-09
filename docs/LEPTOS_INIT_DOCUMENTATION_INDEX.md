# Leptos Init Documentation Index

This directory contains comprehensive documentation for the `leptos init` command and template system.

## 📚 Documentation Files

### User Documentation
- **[LEPTOS_INIT_GUIDE.md](./LEPTOS_INIT_GUIDE.md)** - Complete user guide with examples, templates, and troubleshooting
- **[LEPTOS_INIT_QUICK_REFERENCE.md](./LEPTOS_INIT_QUICK_REFERENCE.md)** - Quick reference card for common commands and patterns

### Developer Documentation
- **[LEPTOS_INIT_DEVELOPER_GUIDE.md](./LEPTOS_INIT_DEVELOPER_GUIDE.md)** - Comprehensive guide for contributors and template developers

## 🎯 Quick Navigation

### For Users
- **Getting Started**: [LEPTOS_INIT_GUIDE.md#quick-start](./LEPTOS_INIT_GUIDE.md#-quick-start)
- **Available Templates**: [LEPTOS_INIT_GUIDE.md#available-templates](./LEPTOS_INIT_GUIDE.md#-available-templates)
- **Command Reference**: [LEPTOS_INIT_GUIDE.md#command-reference](./LEPTOS_INIT_GUIDE.md#-command-reference)
- **Troubleshooting**: [LEPTOS_INIT_GUIDE.md#troubleshooting](./LEPTOS_INIT_GUIDE.md#-troubleshooting)

### For Developers
- **Architecture Overview**: [LEPTOS_INIT_DEVELOPER_GUIDE.md#architecture-overview](./LEPTOS_INIT_DEVELOPER_GUIDE.md#-architecture-overview)
- **Adding New Templates**: [LEPTOS_INIT_DEVELOPER_GUIDE.md#adding-new-templates](./LEPTOS_INIT_DEVELOPER_GUIDE.md#-adding-new-templates)
- **Testing Guidelines**: [LEPTOS_INIT_DEVELOPER_GUIDE.md#testing-guidelines](./LEPTOS_INIT_DEVELOPER_GUIDE.md#-testing-guidelines)
- **Contributing**: [LEPTOS_INIT_DEVELOPER_GUIDE.md#contributing](./LEPTOS_INIT_DEVELOPER_GUIDE.md#-contributing)

## 🚀 Quick Start

### Install and Use
```bash
# Install
cargo install leptos-init

# Create project
leptos init my-app

# Start development
cd my-app
cargo leptos watch
```

### Common Patterns
```bash
# E-commerce site
leptos init shop --template fullstack --database postgresql --styling tailwind

# Personal blog
leptos init blog --template static --styling tailwind

# API service
leptos init api --template api --database sqlite

# Admin dashboard
leptos init admin --template spa --styling tailwind --islands
```

## 📋 Template Overview

| Template | Description | Server | Client | Best For |
|----------|-------------|--------|--------|----------|
| `fullstack` | SSR + Hydration | ✅ Axum | ✅ Hydrate | Web applications |
| `spa` | Client-only | ❌ | ✅ CSR | Static sites, PWAs |
| `api` | Server-only | ✅ Axum | ❌ | Backend services |
| `static` | Static generation | ❌ | ✅ SSR | Blogs, documentation |
| `custom` | Interactive wizard | Configurable | Configurable | Complex requirements |

## 🔧 Configuration Options

### Server Backends
- `axum` - Modern, fast, and ergonomic (default)
- `actix` - High-performance actor framework
- `warp` - Lightweight web server

### Databases
- `sqlite` - File-based, perfect for development
- `postgresql` - Full-featured relational database
- `mysql` - Popular relational database

### Styling Frameworks
- `tailwind` - Utility-first CSS framework
- `vanilla-css` - Plain CSS with custom styles
- `sass` - CSS preprocessor with variables and mixins

### Advanced Features
- `--tracing` - Enable structured logging
- `--islands` - Enable islands architecture
- `--force` - Overwrite existing directories

## 🧪 Testing

The `leptos init` system uses Test-Driven Development (TDD):

```bash
# Run all tests
cargo test --test cli_tests

# Run specific test
cargo test --test cli_tests test_leptos_init_fullstack_template

# Run with output
cargo test --test cli_tests -- --nocapture
```

## 🎯 Benefits

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

## 🔮 Roadmap

### Current Features
- ✅ 5 project templates
- ✅ 3 server backends
- ✅ 3 database options
- ✅ 3 styling frameworks
- ✅ Interactive wizard
- ✅ Comprehensive testing

### Planned Features
- 🔄 Template marketplace
- 🔄 Plugin system
- 🔄 Migration tools
- 🔄 IDE integration
- 🔄 Cloud templates

## 📞 Support

### Getting Help
- **Documentation**: [LEPTOS_INIT_GUIDE.md](./LEPTOS_INIT_GUIDE.md)
- **Quick Reference**: [LEPTOS_INIT_QUICK_REFERENCE.md](./LEPTOS_INIT_QUICK_REFERENCE.md)
- **Discord**: https://discord.gg/YdRAhS7eQB
- **GitHub Issues**: https://github.com/leptos-rs/leptos/issues

### Contributing
- **Developer Guide**: [LEPTOS_INIT_DEVELOPER_GUIDE.md](./LEPTOS_INIT_DEVELOPER_GUIDE.md)
- **Code Style**: Follow Rust conventions
- **Testing**: Use TDD approach
- **Documentation**: Update all relevant docs

---

**Happy coding with Leptos!** 🚀

For the latest updates and community discussions, join our [Discord server](https://discord.gg/YdRAhS7eQB).
