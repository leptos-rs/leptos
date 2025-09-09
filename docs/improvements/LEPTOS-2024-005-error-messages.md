# Framework Error Messages and Developer Feedback

**Issue ID**: LEPTOS-2024-005  
**Severity**: Medium  
**Category**: Developer Experience  
**Status**: Open  
**Created**: 2024-09-08  
**Updated**: 2024-09-08  

## Problem Statement

### Current State
Compiler errors from macros, lifetime issues, and framework misuse often result in cryptic error messages that don't provide actionable guidance for developers, especially those new to Rust or web frameworks.

### Impact Assessment
- **Developer Impact**: 🟡 **Medium** - Frustrating debugging experience, slower development
- **Adoption Impact**: 🔴 **High** - Scares away newcomers, creates perception of difficulty  
- **Maintenance Impact**: 🔴 **High** - High support burden for error interpretation
- **Performance Impact**: ⚪ **None** - No runtime performance impact

### Evidence

**Current Cryptic Errors**:
```rust
// Error: Using signal directly in view
view! { <span>{count}</span> }

// Current error message:
error[E0277]: the trait bound `ReadSignal<i32>: IntoView` is not satisfied
  --> src/main.rs:15:20
   |
15 |     view! { <span>{count}</span> }
   |                    ^^^^^ the trait `IntoView` is not implemented for `ReadSignal<i32>`
   |
   = help: the following other types implement trait `IntoView`:
            ()
            String
            i32
            // ... 50 more lines of unhelpful trait implementations
```

```rust
// Error: Server function without proper feature flags
#[server]  
pub async fn get_data() -> Result<String, ServerFnError> {
    "data".to_string()
}

// Current error message:
error: cannot find function `get_data` in this scope
  --> src/main.rs:25:5
   |
25 |     get_data().await;
   |     ^^^^^^^^ not found in this scope
   |
   = note: server functions are only available when the `ssr` feature is enabled
```

**Community Feedback**:
- "Spent hours debugging trait bounds when I just forgot .get()" 
- "Error messages don't tell me what I should do instead"
- "Coming from JS/TS, Rust errors are intimidating"
- "Need error messages that teach the framework, not just Rust"

## Root Cause Analysis

### Technical Analysis
- Macro-generated code produces low-level Rust errors
- Framework concepts don't map directly to Rust error system
- No custom error handling in macros for common mistakes
- Trait system errors are verbose and unhelpful for framework usage

### Design Analysis
- Framework relies on Rust's error system without customization
- No framework-specific error detection and reporting
- Missing educational component in error messages
- No contextual hints based on common patterns

## Proposed Solution

### Overview
Implement framework-aware error detection and custom error messages that provide clear, actionable guidance with suggestions for fixing common issues.

### Technical Approach

**1. Custom Diagnostic System**
```rust
// Proposed: Framework-specific error detection in macros
view! { <span>{count}</span> }

// Proposed error message:
error: Signal used directly in view without calling .get()
  --> src/main.rs:15:20
   |
15 |     view! { <span>{count}</span> }
   |                    ^^^^^ help: try `count.get()` or `move || count.get()`
   |
   = note: Signals need to be read with .get() to access their values in views
   = help: For dynamic content, use: `{move || count.get()}`
   = help: For one-time reads, use: `{count.get()}`
   = docs: https://leptos.dev/docs/reactivity/signals
```

**2. Context-Aware Suggestions**
```rust
#[server]
pub async fn get_data() -> Result<String, ServerFnError> { ... }

// Usage without proper setup:
get_data().await;

// Proposed error message:
error: Server function 'get_data' called in client context
  --> src/main.rs:25:5
   |
25 |     get_data().await;
   |     ^^^^^^^^ this function only runs on the server
   |
   = help: To load server data on the client, use a Resource:
   |        `let data = Resource::new(|| (), |_| get_data());`
   = help: Then access with: `data.get()`
   = note: Server functions are not directly callable from client code
   = docs: https://leptos.dev/docs/server-functions
```

**3. Configuration Validation**
```rust
// Proposed: Build-time configuration validation
[features]
default = ["csr", "ssr"]  // Invalid combination

// Proposed error message:
error: Conflicting Leptos features enabled
  --> Cargo.toml:15:1
   |
15 | default = ["csr", "ssr"]
   |           ^^^^^^^^^^^^^^ cannot enable both client and server rendering
   |
   = help: Choose one primary rendering mode:
   |        For SPAs: default = ["csr"]
   |        For SSR: default = ["ssr"] 
   |        For SSG: default = ["ssr", "static"]
   = note: Use separate build configurations for different deployment targets
   = docs: https://leptos.dev/docs/deployment
```

**4. Common Pattern Detection**
```rust
// Detect common anti-patterns
create_effect(move |_| {
    if count.get() > 5 {
        set_other.set(true); // Writing to signal from effect
    }
});

// Proposed warning:
warning: Signal update inside effect may cause performance issues
  --> src/main.rs:12:9
   |
12 |         set_other.set(true);
   |         ^^^^^^^^^^^^^^^^^^^ consider using a derived signal instead
   |
   = help: Replace with: `let other = move || count.get() > 5;`
   = note: Effects that update signals can create inefficient update chains
   = docs: https://leptos.dev/docs/common-patterns#derived-signals
```

### Alternative Approaches
1. **IDE Integration**: Rich IDE error support - Limited reach
2. **Separate Checker**: CLI tool for error checking - Extra tool complexity
3. **Runtime Warnings**: Detect issues at runtime - Performance impact

## Implementation Plan

### Phase 1: Foundation (4 weeks)
- [ ] Design custom diagnostic framework
- [ ] Identify common error patterns
- [ ] Create error message templates
- [ ] Implement basic macro error detection

### Phase 2: Implementation (6 weeks)
- [ ] Custom error messages in view! macro
- [ ] Server function validation
- [ ] Configuration checking
- [ ] Pattern-based warnings

### Phase 3: Polish (3 weeks)
- [ ] Error message refinement
- [ ] Documentation links integration
- [ ] Community feedback incorporation
- [ ] IDE integration support

### Success Criteria
- 80% of framework errors provide actionable suggestions
- 60% reduction in error-related support questions
- New developers can resolve common issues independently
- Error messages include learning resources

## Risk Assessment

### Implementation Risks
- **Macro Complexity**: Custom diagnostics add complexity to macros
- **False Positives**: Overly aggressive detection might flag valid patterns
- **Maintenance**: Error messages need updates with framework changes

**Mitigation Strategies**:
- Thorough testing of error detection logic
- Community feedback on error message quality
- Automated testing of error scenarios
- Documentation integration system

### Breaking Changes
✅ **No Breaking Changes** - Only improves error messages

## Testing Strategy

### Unit Tests
- Error detection accuracy
- Message formatting correctness
- Pattern recognition logic

### Integration Tests
- End-to-end error scenarios
- Build system integration
- IDE integration testing

### Performance Tests
- Compilation time impact
- Error processing performance

### User Acceptance Tests
- Error message comprehensibility
- Resolution success rate
- Developer satisfaction with guidance

## Documentation Requirements

### API Documentation
- Error reference guide
- Common error patterns
- Resolution strategies

### User Guides
- "Debugging Leptos Applications"
- "Understanding Framework Errors"
- Error-driven learning materials

### Migration Guides
- None required (improvement only)

## Community Impact

### Backward Compatibility
✅ **Full Compatibility** - Only improves existing behavior

### Learning Curve
📈 **Significant Improvement** - Self-teaching error messages

### Ecosystem Impact
🎯 **Positive** - Reduces support burden, enables independent learning

---

**Related Issues**: All issues benefit from better error messages  
**Dependencies**: Macro system enhancements  
**Assignee**: TBD  
**Milestone**: v0.8.1 (Can be delivered incrementally)