# Feature Flag Mental Overhead

**Issue ID**: LEPTOS-2024-002  
**Severity**: High  
**Category**: Developer Experience  
**Status**: Open  
**Created**: 2024-09-08  
**Updated**: 2024-09-08  

## Problem Statement

### Current State
Developers must understand and correctly configure mutually exclusive feature flags (`csr`, `ssr`, `hydrate`) for different build targets, leading to silent failures and difficult-to-debug issues when misconfigured.

### Impact Assessment
- **Developer Impact**: 🔴 **High** - Silent failures, difficult debugging, project abandonment
- **Adoption Impact**: 🔴 **High** - Confuses new developers, creates perception of complexity
- **Maintenance Impact**: 🟡 **Medium** - High support burden for configuration issues
- **Performance Impact**: ⚪ **None** - When configured correctly

### Evidence

**Current Problematic Pattern**:
```rust
// Different code paths based on feature flags
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    // Server-only code that won't compile on client
    use axum::Router;
    // ... server setup
}

#[cfg(not(feature = "ssr"))]  
pub fn main() {
    // Client-only code
    mount_to_body(App);
}

#[cfg(feature = "hydrate")]
fn special_hydration_logic() {
    // Only runs during hydration
}
```

**Common Configuration Errors**:
```toml
# Error: Multiple conflicting features
[features]
default = ["csr", "ssr"]  # ❌ Breaks build

# Error: Missing feature for target
# cargo build --target wasm32-unknown-unknown
# But no csr feature enabled - silent failure

# Error: Server dependencies on client  
[dependencies]
sqlx = "0.8"  # ❌ Breaks WASM build
```

**Community Feedback**:
- "My app works locally but breaks in production" - Feature flag mismatch
- "Getting 'function not found' errors" - Missing conditional compilation
- "WASM build fails with database errors" - Server deps in client build
- "Tutorial doesn't work" - Feature flag mismatch with example

## Root Cause Analysis

### Technical Analysis
- Feature flags change code compilation paths, creating different binaries
- No validation of feature flag combinations at compile time
- Conditional compilation hides errors until runtime or deployment
- Build process silently excludes code instead of failing fast

### Design Analysis  
- Framework exposes implementation details (rendering modes) as user configuration
- No abstraction layer between build modes and developer intent
- Assumes developers understand client/server architecture complexity
- Lacks smart defaults for common patterns

## Proposed Solution

### Overview
Create intelligent build system that automatically detects target and configures appropriate features, with explicit mode declaration replacing manual feature flag management.

### Technical Approach

**1. Automatic Feature Detection**
```toml
# Proposed: Simple mode declaration
[package.metadata.leptos]
mode = "fullstack"  # Handles csr/ssr/hydrate automatically

# Or explicit per target
mode = { client = "spa", server = "ssr" }
```

**2. Smart Build Integration**
```bash
# cargo-leptos automatically selects correct features
cargo leptos build          # Detects mode, builds both client/server
cargo leptos build --client # Forces CSR build
cargo leptos build --server # Forces SSR build
```

**3. Compile-Time Validation**
```rust
// Proposed: Build-time mode detection
#[leptos::server_only]
async fn database_call() -> Result<Data, Error> {
    // Automatically excluded from client builds
    // Compile error if called from client code
}

#[leptos::client_only]  
fn local_storage_access() {
    // Automatically excluded from server builds
    // Compile error if called from server code
}

#[leptos::universal]
fn shared_logic() {
    // Runs on both client and server
    // Validated at compile time
}
```

**4. Improved Error Messages**
```rust
// Proposed: Clear build errors
error: Server function called in client-only context
  --> src/main.rs:15:5
   |
15 |     database_call().await?;
   |     ^^^^^^^^^^^^^^^ this function only runs on the server
   |
   = help: Move this call to a server context or use a Resource for client-side data loading
   = note: Server functions are not available in CSR (client-side rendering) mode
```

### Alternative Approaches
1. **Build Profiles**: Cargo profile-based solution - Too limited
2. **Workspace Split**: Separate client/server crates - Too complex
3. **Runtime Detection**: Feature detection at runtime - Performance penalty

## Implementation Plan

### Phase 1: Foundation (3 weeks)
- [ ] Design mode declaration system
- [ ] Create build-time feature detection
- [ ] Implement automatic feature selection
- [ ] Basic validation framework

### Phase 2: Implementation (6 weeks)  
- [ ] cargo-leptos integration
- [ ] Compile-time annotations system
- [ ] Error message improvements
- [ ] Cross-compilation validation

### Phase 3: Migration (4 weeks)
- [ ] Backward compatibility layer
- [ ] Migration tooling
- [ ] Example updates
- [ ] Documentation updates

### Success Criteria
- Zero feature flag configuration required for 90% of projects
- Build errors caught at compile time instead of runtime
- 80% reduction in feature-flag related support issues
- Clear error messages with actionable solutions

## Risk Assessment

### Implementation Risks
- **Breaking Changes**: Existing projects need migration path
- **Build Complexity**: Additional build-time logic complexity
- **Edge Cases**: Advanced use cases may not fit new system

**Mitigation Strategies**:
- Maintain backward compatibility for 2+ versions
- Comprehensive test suite for build combinations
- Escape hatches for advanced use cases

### Breaking Changes
🟡 **Medium Breaking Changes**
- Existing feature flag patterns need migration
- Some conditional compilation patterns may need updates
- cargo-leptos command changes

**Migration Path**:
1. Add deprecation warnings for old patterns
2. Provide automatic migration tool
3. Support both systems for transition period
4. Clear migration documentation

## Testing Strategy

### Unit Tests
- Mode detection logic
- Feature flag generation
- Validation rules

### Integration Tests
- All build mode combinations  
- Cross-compilation scenarios
- Cargo integration

### Performance Tests
- Build time impact measurement
- Runtime performance validation

### User Acceptance Tests
- New developer onboarding without feature flags
- Migration experience for existing projects
- Error message clarity

## Documentation Requirements

### API Documentation
- Mode declaration reference
- Build annotation system
- Migration guide from feature flags

### User Guides
- "Building for Different Targets" guide
- Mode selection guide
- Troubleshooting build issues

### Migration Guides
- Feature flag to mode migration
- Breaking changes documentation
- Timeline and support policy

## Community Impact

### Backward Compatibility
🟡 **Breaking Changes Required** - With migration path

### Learning Curve
📈 **Significant Improvement** - Eliminates confusing concept

### Ecosystem Impact
🎯 **Positive** - Enables easier project setup and fewer configuration errors

---

**Related Issues**: LEPTOS-2024-001  
**Dependencies**: cargo-leptos enhancements, build system changes  
**Assignee**: TBD  
**Milestone**: v0.9.0