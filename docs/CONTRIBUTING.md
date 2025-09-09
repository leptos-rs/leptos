# Contributing to Leptos

Thank you for your interest in contributing to Leptos! This comprehensive guide will help you understand the framework's architecture, contribution process, and development workflows.

## Table of Contents
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)  
- [Architecture Overview](#architecture-overview)
- [Contributing Guidelines](#contributing-guidelines)
- [Testing Requirements](#testing-requirements)
- [Code Review Process](#code-review-process)
- [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites

**System Requirements:**
- Rust 1.88+ (workspace MSRV)
- Node.js 18+ (for examples with JS dependencies)
- Git 2.20+

**Required Tools:**
```bash
# Essential tools
cargo install cargo-leptos
cargo install cargo-make
cargo install wasm-pack

# Development tools (recommended)
cargo install cargo-expand
cargo install cargo-nextest
cargo install cargo-audit
```

### Repository Setup

**1. Fork and Clone:**
```bash
# Fork the repository on GitHub
git clone https://github.com/YOUR_USERNAME/leptos.git
cd leptos
```

**2. Set Up Upstream:**
```bash
git remote add upstream https://github.com/leptos-rs/leptos.git
git remote -v
```

**3. Build Workspace:**
```bash
cargo build --workspace
```

**4. Verify Setup:**
```bash
cargo test --workspace
cargo make check
```

## Development Environment

### IDE Configuration

**VS Code (Recommended):**
- Install rust-analyzer extension
- Configure settings.json:
```json
{
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.procMacro.enable": true,
    "editor.formatOnSave": true
}
```

**Vim/Neovim:**
- Use rust-analyzer LSP
- Configure for Rust development

### Code Formatting

**Use nightly for formatting:**
```bash
rustup toolchain install nightly
cargo +nightly fmt
```

**Configure IDE for nightly formatting:**
- Set rust-analyzer to use nightly toolchain for formatting
- Enable format-on-save with nightly

## Architecture Overview

### High-Level Structure

Leptos is organized as a workspace with distinct responsibilities:

```
leptos/
├── Core Framework
│   ├── leptos/              # Main framework crate
│   ├── leptos_macro/        # Procedural macros
│   └── leptos_dom/          # DOM utilities
├── Reactive System
│   ├── reactive_graph/      # Core reactivity
│   └── reactive_stores/     # State management
├── Server Integration  
│   ├── server_fn/           # Server functions
│   ├── leptos_server/       # Server utilities
│   └── integrations/        # Framework integrations
├── Routing & Meta
│   ├── router/              # Client/server routing
│   └── meta/                # HTML head management
└── Rendering Engine
    └── tachys/              # Rendering system
```

### Key Design Principles

**1. Fine-Grained Reactivity**
- Signals track dependencies automatically
- Only affected computations re-run
- No virtual DOM overhead

**2. Isomorphic Architecture**
- Same code runs on client and server
- Server functions bridge client/server gap
- Progressive enhancement support

**3. Performance Focus**
- Zero-cost abstractions where possible
- Minimal WASM bundle sizes
- Efficient DOM updates

**4. Type Safety**
- Leverages Rust's type system
- Compile-time guarantees
- Minimal runtime errors

### Component Lifecycle

```rust
// 1. Component definition
#[component]
fn MyComponent() -> impl IntoView {
    // 2. Signal creation (reactive state)
    let (count, set_count) = signal(0);
    
    // 3. Effect creation (side effects)
    effect(move |_| {
        log!("Count: {}", count.get());
    });
    
    // 4. View generation
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            {count}
        </button>
    }
}
```

## Contributing Guidelines

### Types of Contributions

**Bug Fixes**
- Check existing issues first
- Include reproduction steps
- Add regression tests
- Keep changes minimal and focused

**New Features**
- Discuss in GitHub Discussions or Discord first
- Consider impact on bundle size and performance
- Follow existing API patterns
- Include comprehensive tests and documentation

**Documentation**
- Fix typos and improve clarity
- Add missing examples
- Update API documentation
- Improve getting started guides

**Performance Improvements**
- Benchmark before and after
- Consider both client and server performance
- Document the optimization approach
- Ensure no breaking changes

### Code Standards

**Rust Style Guidelines:**
- Follow official Rust style guide
- Use `cargo +nightly fmt` before committing
- Pass `cargo clippy` without warnings
- Add documentation for public APIs

**API Design:**
- Consistent naming conventions
- Prefer explicit over implicit
- Maintain backward compatibility when possible
- Use builder patterns for complex configuration

**Error Handling:**
- Use `Result<T, E>` for fallible operations
- Provide meaningful error messages
- Don't panic in library code
- Include context in error types

### Commit Guidelines

**Commit Message Format:**
```
type(scope): short description

Longer description if needed.
Explain the why, not just what.

Fixes #issue_number
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `chore`: Build process or auxiliary tool changes

**Example:**
```
feat(router): add lazy route loading

Implement lazy loading for routes to reduce initial bundle size.
Routes are now loaded on-demand when first accessed.

- Add LazyRoute component
- Update router to handle async route loading
- Add loading states for route transitions

Fixes #1234
```

## Testing Requirements

### Test Categories

**1. Unit Tests**
```bash
# Run unit tests for specific crate
cargo test -p leptos

# Run with features
cargo test --features "ssr"
```

**2. Integration Tests**
```bash
# Run integration tests
cargo test --test integration

# Run examples tests
cargo make check-examples
```

**3. WASM Tests**
```bash
# Browser testing
wasm-pack test --headless --chrome leptos_dom
```

**4. End-to-End Tests**
```bash
# Example-specific E2E tests
cd examples/todo_app_sqlite_axum/e2e
cargo test
```

### Writing Tests

**Component Testing:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use leptos::*;

    #[test]
    fn test_counter_increment() {
        let runtime = create_runtime();
        
        let (count, set_count) = create_signal(0);
        
        // Test initial state
        assert_eq!(count.get(), 0);
        
        // Test increment
        set_count.update(|n| *n += 1);
        assert_eq!(count.get(), 1);
        
        runtime.dispose();
    }
}
```

**Server Function Testing:**
```rust
#[tokio::test]
async fn test_server_function() {
    // Setup test environment
    let result = my_server_function(test_input).await;
    
    assert!(result.is_ok());
    // Additional assertions
}
```

### Benchmark Testing

**Performance Benchmarks:**
```bash
cd benchmarks
cargo bench
```

**Custom Benchmarks:**
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
```

### Pre-Submission Checklist

**Before submitting a PR:**
```bash
# Format code
cargo +nightly fmt

# Check code quality
cargo +nightly clippy --workspace --all-targets --all-features

# Run tests
cargo test --workspace

# Check examples
cargo make check-examples

# Full CI check
cargo +nightly make --profile=github-actions ci
```

## Code Review Process

### PR Requirements

**Every PR should include:**
- Clear description of changes
- Link to related issue (if applicable)
- Tests for new functionality
- Documentation updates
- No breaking changes (unless discussed)

**PR Description Template:**
```markdown
## Summary
Brief description of what this PR does.

## Changes
- List of specific changes made
- Any breaking changes and migration path

## Testing
- How you tested the changes
- New tests added

## Related Issues
Fixes #issue_number
```

### Review Criteria

**Code Quality:**
- Follows Rust best practices
- Consistent with existing codebase
- Proper error handling
- Adequate test coverage

**Performance:**
- No significant performance regressions
- Bundle size impact considered
- Memory usage implications

**Documentation:**
- Public APIs documented
- Examples updated if needed
- Clear error messages

### Addressing Review Feedback

**When changes are requested:**
1. Address each comment individually
2. Ask for clarification if unclear
3. Update PR description if scope changes
4. Re-run tests after changes
5. Mark conversations as resolved when addressed

## Community Guidelines

### Communication Channels

**GitHub Issues:**
- Bug reports and feature requests
- Technical discussions
- Documentation improvements

**GitHub Discussions:**
- Design discussions
- Architecture questions
- Community showcase

**Discord Server:**
- Real-time help and support
- Community chat
- Development coordination
- Link: https://discord.gg/YdRAhS7eQB

### Code of Conduct

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct):

- Be welcoming and inclusive
- Be respectful and considerate  
- Communicate constructively
- Ask for help when needed

### Getting Help

**Stuck on something?**
1. Check existing documentation and examples
2. Search GitHub issues and discussions
3. Ask on Discord #help channel
4. Create a detailed GitHub issue

**Want to contribute but don't know where to start?**
- Look for "good first issue" labels
- Check "help wanted" labels  
- Ask on Discord what needs work
- Improve documentation or examples

### Recognition

**Contributors are recognized through:**
- GitHub contributor statistics
- Release notes acknowledgments
- Community showcases
- Maintainer recommendations

### Advanced Contributing

**Becoming a Maintainer:**
- Consistent quality contributions
- Help with community support
- Understand framework architecture
- Express interest to current maintainers

**Working on Core Features:**
- Discuss design in GitHub Discussions first
- Create RFC for major changes
- Coordinate with maintainers
- Consider backward compatibility

## Additional Resources

### Learning Resources
- **Leptos Book**: https://leptos-rs.github.io/leptos/
- **API Documentation**: https://docs.rs/leptos/
- **Example Applications**: `./examples/` directory

### Development Tools
- **cargo-leptos**: Official build tool
- **trunk**: Alternative development server
- **wasm-pack**: WASM building and testing

### Community Resources
- **Awesome Leptos**: https://github.com/leptos-rs/awesome-leptos
- **Community Showcase**: GitHub Discussions
- **Newsletter**: Subscribe for updates

---

Thank you for contributing to Leptos! Your efforts help make web development in Rust better for everyone.