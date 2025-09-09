# Leptos Init Command Implementation Strategy

## Current State Analysis

### Existing Tools
- ✅ `cargo-leptos` exists with basic `new` command (using cargo-generate)
- ✅ Complex example configurations available (counter_isomorphic: 90 lines)
- ❌ No unified template system for common patterns
- ❌ Manual feature flag configuration required
- ❌ No intelligent defaults for project types

### P0 Critical Issues Identified
1. **Setup Complexity**: 30+ minutes vs competitors' <5 minutes
2. **Feature Flag Confusion**: Silent failures from misconfiguration
3. **Adoption Barrier**: Primary reason for framework abandonment

## Implementation Strategy

### Phase 1: Enhanced Template System (Week 1-2)

**Objective**: Create intelligent template system that eliminates manual configuration

**Implementation**:
```rust
// New leptos init command structure
pub enum ProjectTemplate {
    Spa,        // Client-side only
    Fullstack,  // Server + Client with hydration  
    Static,     // Static site generation
    Api,        // Server functions only
    Custom,     // Interactive wizard
}

pub struct InitConfig {
    name: String,
    template: ProjectTemplate,
    server: ServerBackend,  // Axum, Actix, etc.
    database: Option<Database>, // SQLite, PostgreSQL, etc.
    styling: Option<Styling>,   // Tailwind, vanilla CSS, etc.
}
```

**Smart Templates**:
```toml
# Generated Cargo.toml for fullstack template
[package]
name = "my-app"
version = "0.1.0" 
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.8", features = ["tracing"] }
leptos_axum = { version = "0.8", optional = true }
# ... auto-generated based on template

[features]
default = []
hydrate = ["leptos/hydrate"]
ssr = ["dep:leptos_axum", "leptos/ssr"]

[package.metadata.leptos]
template = "fullstack"
mode = "auto"  # Intelligent mode selection
```

### Phase 2: Intelligent Feature Resolution (Week 3-4)

**Objective**: Eliminate feature flag confusion with automatic detection

**Key Innovation**: Mode-based configuration instead of manual feature flags
```toml
[package.metadata.leptos]
mode = "fullstack"  # Replaces complex feature flag combinations

# OR explicit per-target
[package.metadata.leptos.modes]
client = "spa"      # Automatically enables ["csr"]
server = "ssr"      # Automatically enables ["ssr", "hydrate"]
```

**Build System Integration**:
```rust
// Enhanced cargo-leptos build logic
impl BuildMode {
    fn resolve_features(&self, target: &Target) -> Vec<String> {
        match (self, target) {
            (BuildMode::Fullstack, Target::Client) => vec!["hydrate"],
            (BuildMode::Fullstack, Target::Server) => vec!["ssr"],
            (BuildMode::Spa, Target::Client) => vec!["csr"],
            (BuildMode::Spa, Target::Server) => panic!("SPA mode has no server target"),
            // ... intelligent resolution
        }
    }
}
```

### Phase 3: Compile-Time Validation (Week 5-6)

**Objective**: Catch configuration errors at compile time, not runtime

**New Annotation System**:
```rust
#[leptos::server_only]
async fn database_query() -> Result<Data, Error> {
    // Compile error if called from client context
}

#[leptos::client_only] 
fn local_storage_access() {
    // Compile error if called from server context
}

#[leptos::universal]
fn shared_logic() {
    // Validated to work in both contexts
}
```

**Enhanced Error Messages**:
```
error: Server function called in client-only context
  --> src/app.rs:42:5
   |
42 |     database_query().await?;
   |     ^^^^^^^^^^^^^^^ this function only runs on the server
   |
   = help: Use a Resource or server action to load data on the client
   = note: Current build mode: SPA (client-side only)
```

## TDD Implementation Plan

### Test-Driven Development Cycle

**1. Problem Validation Tests**
```rust
#[test]
fn test_current_setup_complexity() {
    // Validate that current setup takes 30+ steps
    let steps = count_manual_setup_steps();
    assert!(steps > 30, "Setup should be complex currently");
}

#[test] 
fn test_feature_flag_confusion() {
    // Validate that conflicting flags cause silent failures
    let config = CargoToml::with_features(["csr", "ssr"]);
    assert!(config.build().is_err(), "Should fail with conflicting features");
}
```

**2. Solution Design Tests**
```rust
#[test]
fn test_init_reduces_setup_steps() {
    let result = leptos_init("my-app", Template::Fullstack);
    assert!(result.setup_steps() < 5, "Should require <5 steps");
}

#[test]
fn test_automatic_feature_resolution() {
    let config = InitConfig::new(Template::Fullstack);
    assert_eq!(config.client_features(), vec!["hydrate"]);
    assert_eq!(config.server_features(), vec!["ssr"]);
}
```

**3. Implementation Tests**
```rust
#[test]
fn test_template_generation() {
    let project = generate_project("my-app", Template::Fullstack);
    assert!(project.builds_successfully());
    assert!(project.runs_in_development());
    assert!(project.deploys_to_production());
}
```

## Success Metrics

### Quantitative Goals
- ✅ Setup time: 30+ minutes → <5 minutes (85% reduction)
- ✅ Configuration lines: 90+ → <20 (75% reduction)  
- ✅ Support issues: Reduce feature-flag questions by 80%
- ✅ Compile-time error rate: 90%+ of issues caught at build time

### Qualitative Goals
- ✅ New developer onboarding without framework expertise
- ✅ Zero manual feature flag configuration 
- ✅ Clear, actionable error messages
- ✅ Backward compatibility with existing projects

## Risk Mitigation

### Breaking Changes
- **Risk**: Existing projects need migration
- **Mitigation**: Maintain backward compatibility for 2+ versions
- **Plan**: Provide automatic migration tooling

### Build Complexity  
- **Risk**: Additional build-time logic increases complexity
- **Mitigation**: Comprehensive test suite for all build combinations
- **Plan**: Performance monitoring to ensure <10% build time increase

### Edge Cases
- **Risk**: Advanced use cases may not fit new system
- **Mitigation**: Escape hatches for manual configuration
- **Plan**: Community feedback loop for missing patterns

## Implementation Timeline

| Week | Phase | Deliverable |
|------|-------|-------------|
| 1-2  | Template System | Enhanced `leptos init` with smart templates |
| 3-4  | Feature Resolution | Automatic mode-based feature selection |
| 5-6  | Validation | Compile-time error detection and messaging |
| 7-8  | Integration | cargo-leptos integration and testing |

## Next Steps

1. **Implement Template System**: Start with basic template generation
2. **Create Test Suite**: TDD approach with comprehensive coverage
3. **Community Validation**: Get feedback on template patterns
4. **Integration Testing**: Ensure compatibility across platforms

---

**Status**: Ready for implementation  
**Priority**: P0 Critical  
**Impact**: Addresses primary adoption barriers