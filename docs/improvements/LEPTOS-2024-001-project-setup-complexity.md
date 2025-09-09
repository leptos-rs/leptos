# Project Setup Complexity

**Issue ID**: LEPTOS-2024-001  
**Severity**: Critical  
**Category**: Developer Experience  
**Status**: Open  
**Created**: 2024-09-08  
**Updated**: 2024-09-08  

## Problem Statement

### Current State
New developers must manually configure complex `Cargo.toml` files with 20+ configuration options, feature flags, and conditional dependencies to create a working Leptos application. Full-stack examples require 90+ line configuration files with intricate feature flag combinations.

### Impact Assessment
- **Developer Impact**: 🔴 **Critical** - New developers abandon framework during initial setup
- **Adoption Impact**: 🔴 **Critical** - Primary barrier to framework adoption
- **Maintenance Impact**: 🟡 **Medium** - High support burden for setup issues
- **Performance Impact**: ⚪ **None** - No direct performance impact

### Evidence
```toml
# Current required configuration for full-stack app
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { path = "../../leptos", features = ["tracing"] }
leptos_axum = { path = "../../integrations/axum", optional = true }
axum = { version = "0.8.1", optional = true }
tower = { version = "0.4.13", optional = true }
tower-http = { version = "0.5.2", features = ["fs"], optional = true }
tokio = { version = "1.39", features = ["full"], optional = true }
sqlx = { version = "0.8.6", features = ["runtime-tokio-rustls", "sqlite"], optional = true }

[features]
hydrate = ["leptos/hydrate"]
ssr = ["dep:axum", "dep:tower", "dep:tower-http", "dep:tokio", "dep:sqlx", "leptos/ssr", "dep:leptos_axum"]

[package.metadata.leptos]
output-name = "my-app"
site-root = "target/site"
site-pkg-dir = "pkg"
site-addr = "127.0.0.1:3000"
reload-port = 3001
bin-features = ["ssr"]
bin-default-features = false
lib-features = ["hydrate"]
lib-default-features = false
# ... 10+ more required options
```

**Community Feedback**:
- "Spent 2 hours trying to get first example running" - Discord #help channel
- "Configuration is more complex than the actual app code" - GitHub discussion
- "Need a create-leptos-app equivalent" - Multiple feature requests

## Root Cause Analysis

### Technical Analysis
- No official project scaffolding tool
- Feature flag system requires deep framework knowledge
- Manual dependency management for each rendering mode
- Complex metadata configuration with non-obvious defaults

### Design Analysis
- Framework prioritizes flexibility over ease of use
- No opinionated defaults for common use cases
- Configuration complexity reflects internal architecture
- Missing abstraction layer for project initialization

## Proposed Solution

### Overview
Create `leptos init` command that generates complete, working project configurations based on selected templates and automatically handles all setup complexity.

### Technical Approach

**1. CLI Tool Enhancement**
- Extend `cargo-leptos` with `init` subcommand
- Interactive template selection
- Automatic dependency resolution
- Environment-specific configuration generation

**2. Template System**
```bash
leptos init my-app --template fullstack
# Generates complete Axum + SQLite setup

leptos init my-app --template spa  
# Client-side only setup

leptos init my-app --template static
# Static site generation setup

leptos init my-app --template api
# Server functions only
```

**3. Smart Defaults**
```toml
# Generated simple configuration
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

# All complexity hidden in template
[dependencies]
leptos = { version = "0.8", features = ["default"] }

# Leptos handles the rest automatically
[package.metadata.leptos]
template = "fullstack"
```

### Alternative Approaches
1. **Configuration Wizard**: Interactive CLI setup - Rejected due to complexity
2. **Web-based Generator**: Browser-based setup - Rejected due to offline requirements  
3. **Cargo Extensions**: Extend cargo new - Rejected due to limited customization

## Implementation Plan

### Phase 1: Foundation (2 weeks)
- [ ] Design template system architecture
- [ ] Create template definitions for common patterns
- [ ] Implement basic `leptos init` command
- [ ] Add template selection logic

### Phase 2: Implementation (4 weeks)
- [ ] Generate complete project structure
- [ ] Automatic dependency resolution
- [ ] Configuration file generation
- [ ] Basic project validation

### Phase 3: Polish (2 weeks)
- [ ] Interactive template customization
- [ ] Dependency version management
- [ ] Error handling and validation
- [ ] Documentation and examples

### Success Criteria
- New project setup time reduced from 30+ minutes to <5 minutes
- Zero manual configuration required for common use cases
- 90% reduction in setup-related support questions
- Template covers 80% of common project patterns

## Risk Assessment

### Implementation Risks
- **Template Maintenance**: Templates must stay current with framework changes
- **Complexity Creep**: Avoiding recreating configuration complexity in templates
- **Compatibility**: Ensuring generated projects work across platforms

**Mitigation Strategies**:
- Automated template testing in CI
- Version pinning for template dependencies
- Platform-specific template variants

### Breaking Changes
- No breaking changes to existing projects
- New optional tooling only
- Backward compatibility maintained

## Testing Strategy

### Unit Tests
- Template generation logic
- Configuration file creation
- Dependency resolution

### Integration Tests
- Generated project compilation
- Full development workflow testing
- Cross-platform compatibility

### Performance Tests
- Template generation speed
- Project setup time measurement

### User Acceptance Tests
- New developer onboarding flow
- Template completeness validation
- Setup time measurement

## Documentation Requirements

### API Documentation
- `leptos init` command reference
- Template system documentation
- Configuration options reference

### User Guides
- "Getting Started" guide update
- Template selection guide
- Project structure explanation

### Migration Guides
- None required (new feature)

## Community Impact

### Backward Compatibility
✅ **No Impact** - Purely additive feature

### Learning Curve
📈 **Significant Improvement** - Eliminates primary barrier to entry

### Ecosystem Impact
🎯 **Positive** - Enables faster community growth and adoption

---

**Related Issues**: None  
**Dependencies**: cargo-leptos enhancements  
**Assignee**: TBD  
**Milestone**: v0.9.0