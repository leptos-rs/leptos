# Leptos Mode CLI

A command-line tool for detecting, analyzing, and migrating Leptos projects to use the new automatic mode detection system.

## Installation

```bash
cargo install --path leptos_mode_cli
```

## Usage

### Analyze Project

Analyze your current Leptos project to detect the appropriate mode and identify configuration issues:

```bash
# Basic analysis
leptos-mode analyze

# Verbose output with detailed recommendations
leptos-mode analyze --verbose

# Output in JSON format
leptos-mode analyze --format json

# Analyze a specific directory
leptos-mode analyze --path /path/to/project
```

### Migrate Project

Automatically migrate your project to use the new mode detection system:

```bash
# Interactive migration with confirmation
leptos-mode migrate

# Force migration without confirmation
leptos-mode migrate --force

# Migrate without creating backup
leptos-mode migrate --no-backup
```

### Validate Configuration

Validate your current project configuration:

```bash
# Check for configuration issues
leptos-mode validate

# Attempt to fix issues automatically
leptos-mode validate --fix
```

### Generate Configuration

Generate build configuration for a specific mode:

```bash
# Generate SPA mode configuration
leptos-mode generate --mode spa

# Generate fullstack configuration for production
leptos-mode generate --mode fullstack --env production

# Save configuration to file
leptos-mode generate --mode static --output leptos.toml
```

### Get Help

Get detailed help for specific modes:

```bash
# Get help for SPA mode
leptos-mode help spa

# Get help for fullstack mode
leptos-mode help fullstack
```

## Modes

### SPA (Single Page Application)
- **Use case**: Client-side only applications
- **Features**: CSR (Client-Side Rendering)
- **Best for**: Static sites, prototypes, client-only apps

### Fullstack
- **Use case**: Full-stack web applications
- **Features**: SSR + Hydration
- **Best for**: Production apps, SEO-important sites

### Static
- **Use case**: Static site generation
- **Features**: Pre-rendered HTML
- **Best for**: Documentation, blogs, marketing pages

### API
- **Use case**: Server-only applications
- **Features**: Server-side rendering, API endpoints
- **Best for**: REST APIs, backend services

## Migration Process

The migration process will:

1. **Analyze** your current project structure and configuration
2. **Detect** the appropriate mode based on your code patterns
3. **Identify** configuration issues and conflicts
4. **Generate** recommendations for improvement
5. **Apply** changes to migrate to the new system
6. **Setup** automatic validation

### What Gets Changed

- Updates `Cargo.toml` with mode declarations
- Removes manual feature flag configurations
- Adds automatic validation setup
- Creates backup of original files (optional)

### Example Migration

```bash
$ leptos-mode analyze
🔍 Analyzing Leptos project...

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

$ leptos-mode migrate
🚀 Migrating project to automatic mode detection...

📋 Migration Plan:
  1. Add mode declaration
  2. Remove conflicting features

Do you want to proceed with the migration? [Y/n]: y
📦 Backup created at .leptos-backup/
  Applying: Add mode declaration
  Applying: Remove conflicting features

✅ Migration completed successfully!
Run 'cargo check' to verify the changes.
```

## Configuration

After migration, your `Cargo.toml` will include:

```toml
[package.metadata.leptos]
mode = "fullstack"
env = "DEV"
```

## Validation

The tool automatically sets up compile-time validation to prevent configuration errors:

- Feature flag conflicts
- Context mismatches (server vs client code)
- Invalid mode configurations
- Missing required features

## Troubleshooting

### Common Issues

1. **"Conflicting features detected"**
   - Solution: Use `leptos-mode migrate` to automatically resolve conflicts

2. **"Invalid mode for target"**
   - Solution: Check that your mode matches your build target (client vs server)

3. **"Missing required features"**
   - Solution: The tool will suggest the correct features for your mode

### Getting Help

- Run `leptos-mode help <mode>` for mode-specific help
- Check the [Leptos documentation](https://leptos.dev) for detailed guides
- Open an issue on GitHub for bugs or feature requests

## Contributing

Contributions are welcome! Please see the main Leptos repository for contribution guidelines.
