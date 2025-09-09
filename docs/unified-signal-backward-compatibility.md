# Unified Signal API - Backward Compatibility Plan

## Overview

This document outlines the backward compatibility strategy for the unified signal API in Leptos. The goal is to ensure a smooth transition from the current signal APIs to the new unified API while maintaining full backward compatibility and providing clear migration paths.

## Compatibility Principles

### 1. Non-Breaking Changes
- All existing APIs remain functional
- No breaking changes to public interfaces
- Existing code continues to work without modification

### 2. Gradual Migration
- New API available alongside existing APIs
- Clear deprecation timeline
- Automated migration tools

### 3. Performance Parity
- New API maintains or improves performance
- No performance regressions
- Zero-cost abstractions where possible

## Compatibility Timeline

### Phase 1: Additive (v0.9.0)
- ✅ New unified API introduced
- ✅ All existing APIs remain functional
- ✅ No deprecation warnings
- ✅ Full backward compatibility
- ✅ New projects can use unified API

### Phase 2: Deprecation (v0.10.0)
- ⚠️ Deprecation warnings for old APIs
- 🔧 Migration tools available
- 📚 Documentation updated
- 🎯 Community examples use unified API

### Phase 3: Removal (v1.0.0)
- 🗑️ Deprecated APIs removed
- 🚀 Complete migration to unified API
- 📖 Legacy documentation archived

## API Compatibility Matrix

### Current APIs → New APIs

| Current API | New API | Compatibility | Migration |
|-------------|---------|---------------|-----------|
| `create_signal(0)` | `signal(0)` | ✅ Direct | Drop-in replacement |
| `create_rw_signal(0)` | `signal(0)` | ✅ Direct | Drop-in replacement |
| `create_memo(|| 0)` | `signal(0).derive(\|v\| 0)` | ✅ Direct | Method chaining |
| `create_resource(|| async { 0 })` | `signal::async(|| (), \|_\| async { 0 })` | ✅ Direct | Function change |
| `signal.get()` | `signal.get()` | ✅ Direct | No change |
| `set_signal(42)` | `signal.set(42)` | ✅ Direct | Method instead of function |
| `set_signal.update(\|v\| *v += 1)` | `signal.update(\|v\| *v += 1)` | ✅ Direct | Method instead of function |

### Type Compatibility

| Current Type | New Type | Compatibility | Notes |
|--------------|----------|---------------|-------|
| `ReadSignal<T>` | `impl Signal<T>` | ✅ Compatible | Trait implementation |
| `WriteSignal<T>` | `impl Signal<T>` | ✅ Compatible | Trait implementation |
| `RwSignal<T>` | `impl Signal<T>` | ✅ Compatible | Trait implementation |
| `Memo<T>` | `impl Signal<T>` | ✅ Compatible | Trait implementation |
| `Resource<T>` | `impl Signal<T>` | ✅ Compatible | Trait implementation |

## Implementation Strategy

### 1. Trait-Based Compatibility

```rust
// Current APIs remain unchanged
pub fn create_signal<T>(cx: Scope, initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    // Existing implementation
}

pub fn create_rw_signal<T>(cx: Scope, initial: T) -> RwSignal<T> {
    // Existing implementation
}

// New unified API
pub fn signal<T>(cx: Scope, initial: T) -> impl Signal<T> {
    // New implementation that wraps existing APIs
    let (read, write) = create_signal(cx, initial);
    UnifiedSignal { read, write }
}

// Unified signal trait
pub trait Signal<T: Clone + 'static>: Clone + 'static {
    fn get(&self) -> T;
    fn set(&self, value: T);
    fn update(&self, f: impl FnOnce(&mut T));
    fn derive<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<U>;
    fn split(self) -> (impl ReadSignal<T>, impl WriteSignal<T>);
}

// Implementation for existing types
impl<T: Clone + 'static> Signal<T> for RwSignal<T> {
    fn get(&self) -> T { self.get() }
    fn set(&self, value: T) { self.set(value) }
    fn update(&self, f: impl FnOnce(&mut T)) { self.update(f) }
    fn derive<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<U> {
        // Implementation
    }
    fn split(self) -> (impl ReadSignal<T>, impl WriteSignal<T>) {
        (self.read_only(), self.write_only())
    }
}
```

### 2. Wrapper-Based Compatibility

```rust
// Wrapper for existing signal types
pub struct UnifiedSignal<T: Clone + 'static> {
    inner: Box<dyn SignalImpl<T>>,
}

trait SignalImpl<T: Clone + 'static> {
    fn get(&self) -> T;
    fn set(&self, value: T);
    fn update(&self, f: impl FnOnce(&mut T));
    fn derive<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Box<dyn SignalImpl<U>>;
    fn split(self: Box<Self>) -> (ReadSignal<T>, WriteSignal<T>);
}

// Implementation for RwSignal
impl<T: Clone + 'static> SignalImpl<T> for RwSignal<T> {
    fn get(&self) -> T { self.get() }
    fn set(&self, value: T) { self.set(value) }
    fn update(&self, f: impl FnOnce(&mut T)) { self.update(f) }
    fn derive<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Box<dyn SignalImpl<U>> {
        // Implementation
    }
    fn split(self: Box<Self>) -> (ReadSignal<T>, WriteSignal<T>) {
        // Implementation
    }
}
```

### 3. Enum-Based Compatibility

```rust
// Enum for different signal types
pub enum UnifiedSignal<T: Clone + 'static> {
    RwSignal(RwSignal<T>),
    Memo(Memo<T>),
    Resource(Resource<T>),
    Custom(Box<dyn SignalImpl<T>>),
}

impl<T: Clone + 'static> Signal<T> for UnifiedSignal<T> {
    fn get(&self) -> T {
        match self {
            UnifiedSignal::RwSignal(s) => s.get(),
            UnifiedSignal::Memo(s) => s.get(),
            UnifiedSignal::Resource(s) => s.get(),
            UnifiedSignal::Custom(s) => s.get(),
        }
    }
    
    fn set(&self, value: T) {
        match self {
            UnifiedSignal::RwSignal(s) => s.set(value),
            UnifiedSignal::Memo(_) => panic!("Cannot set a memo signal"),
            UnifiedSignal::Resource(_) => panic!("Cannot set a resource signal"),
            UnifiedSignal::Custom(s) => s.set(value),
        }
    }
    
    // ... other methods
}
```

## Migration Tools

### 1. Automated Migration Script

```rust
// Migration tool for automatic code conversion
pub struct MigrationTool {
    // Configuration
    config: MigrationConfig,
}

pub struct MigrationConfig {
    pub dry_run: bool,
    pub backup: bool,
    pub verbose: bool,
    pub patterns: Vec<MigrationPattern>,
}

pub enum MigrationPattern {
    CreateSignalToSignal,
    CreateRwSignalToSignal,
    CreateMemoToDerive,
    CreateResourceToAsync,
    SetterToMethod,
    UpdateToMethod,
}

impl MigrationTool {
    pub fn migrate(&self, path: &Path) -> Result<MigrationResult> {
        // Parse Rust code
        let syntax_tree = syn::parse_file(&fs::read_to_string(path)?)?;
        
        // Apply migration patterns
        let migrated = self.apply_patterns(syntax_tree);
        
        // Generate migration report
        let report = self.generate_report(&syntax_tree, &migrated);
        
        // Apply changes if not dry run
        if !self.config.dry_run {
            fs::write(path, quote::quote!(#migrated).to_string())?;
        }
        
        Ok(MigrationResult { report, migrated })
    }
}
```

### 2. Interactive Migration Assistant

```rust
// Interactive migration with user confirmation
pub struct InteractiveMigration {
    tool: MigrationTool,
    ui: MigrationUI,
}

impl InteractiveMigration {
    pub fn run(&self, path: &Path) -> Result<()> {
        let result = self.tool.migrate(path)?;
        
        // Show changes to user
        self.ui.show_changes(&result.report);
        
        // Get user confirmation
        if self.ui.confirm_changes()? {
            self.tool.apply_changes(path)?;
            self.ui.show_success();
        } else {
            self.ui.show_cancelled();
        }
        
        Ok(())
    }
}
```

### 3. Migration Validation

```rust
// Validate migration results
pub struct MigrationValidator {
    // Validation rules
    rules: Vec<ValidationRule>,
}

pub enum ValidationRule {
    PerformanceCheck,
    FunctionalityCheck,
    CompilationCheck,
    TestCheck,
}

impl MigrationValidator {
    pub fn validate(&self, before: &Path, after: &Path) -> Result<ValidationResult> {
        let mut results = Vec::new();
        
        for rule in &self.rules {
            match rule {
                ValidationRule::PerformanceCheck => {
                    let perf_result = self.check_performance(before, after)?;
                    results.push(perf_result);
                }
                ValidationRule::FunctionalityCheck => {
                    let func_result = self.check_functionality(before, after)?;
                    results.push(func_result);
                }
                ValidationRule::CompilationCheck => {
                    let comp_result = self.check_compilation(after)?;
                    results.push(comp_result);
                }
                ValidationRule::TestCheck => {
                    let test_result = self.check_tests(after)?;
                    results.push(test_result);
                }
            }
        }
        
        Ok(ValidationResult { results })
    }
}
```

## Deprecation Strategy

### 1. Deprecation Warnings

```rust
// Deprecation attributes with helpful messages
#[deprecated(
    since = "0.10.0",
    note = "Use `signal()` instead. Run `leptos-migrate` to automatically convert your code."
)]
pub fn create_signal<T>(cx: Scope, initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    // Implementation
}

#[deprecated(
    since = "0.10.0",
    note = "Use `signal().derive()` instead. Run `leptos-migrate` to automatically convert your code."
)]
pub fn create_memo<T>(cx: Scope, f: impl Fn() -> T + 'static) -> Memo<T> {
    // Implementation
}
```

### 2. Gradual Deprecation

```rust
// Phase 1: Soft deprecation (warnings only)
#[deprecated(since = "0.10.0", note = "Use `signal()` instead")]
pub fn create_signal<T>(cx: Scope, initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    // Implementation
}

// Phase 2: Hard deprecation (compilation errors)
#[deprecated(since = "0.11.0", note = "Use `signal()` instead. This API will be removed in v1.0.0")]
pub fn create_signal<T>(cx: Scope, initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    // Implementation
}

// Phase 3: Removal (API no longer exists)
// create_signal function removed entirely
```

### 3. Migration Assistance

```rust
// Compiler suggestions for migration
pub fn create_signal<T>(cx: Scope, initial: T) -> (ReadSignal<T>, WriteSignal<T>) {
    // Compiler suggestion
    #[cfg(feature = "deprecated")]
    compile_error!(
        "`create_signal` is deprecated. Use `signal()` instead.\n\
         Run `leptos-migrate` to automatically convert your code.\n\
         See https://leptos.dev/migration for more information."
    );
    
    // Implementation
}
```

## Testing Strategy

### 1. Compatibility Tests

```rust
#[cfg(test)]
mod compatibility_tests {
    use super::*;
    
    #[test]
    fn test_old_api_still_works() {
        // Test that old APIs still function
        let (read, write) = create_signal(0);
        assert_eq!(read.get(), 0);
        write.set(42);
        assert_eq!(read.get(), 42);
    }
    
    #[test]
    fn test_new_api_works() {
        // Test that new API works
        let signal = signal(0);
        assert_eq!(signal.get(), 0);
        signal.set(42);
        assert_eq!(signal.get(), 42);
    }
    
    #[test]
    fn test_mixed_usage() {
        // Test mixing old and new APIs
        let (old_read, old_write) = create_signal(0);
        let new_signal = signal(0);
        
        old_write.set(42);
        new_signal.set(42);
        
        assert_eq!(old_read.get(), new_signal.get());
    }
}
```

### 2. Migration Tests

```rust
#[cfg(test)]
mod migration_tests {
    use super::*;
    
    #[test]
    fn test_migration_tool() {
        let tool = MigrationTool::new();
        let result = tool.migrate("test_code.rs").unwrap();
        
        // Verify migration results
        assert!(result.report.changes.len() > 0);
        assert!(result.migrated.contains("signal("));
        assert!(!result.migrated.contains("create_signal("));
    }
    
    #[test]
    fn test_migration_validation() {
        let validator = MigrationValidator::new();
        let result = validator.validate("before.rs", "after.rs").unwrap();
        
        // Verify validation results
        assert!(result.results.iter().all(|r| r.is_success()));
    }
}
```

### 3. Performance Tests

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    
    #[test]
    fn test_performance_parity() {
        // Benchmark old API
        let old_time = benchmark_old_api();
        
        // Benchmark new API
        let new_time = benchmark_new_api();
        
        // Ensure performance is within acceptable limits
        assert!(new_time <= old_time * 1.1); // 10% overhead max
    }
}
```

## Documentation Strategy

### 1. Migration Guide

```markdown
# Migration Guide

## Quick Start
1. Install migration tool: `cargo install leptos-migrate`
2. Run migration: `leptos-migrate --path ./src`
3. Review changes: `leptos-migrate --dry-run`
4. Apply changes: `leptos-migrate --apply`

## Manual Migration
- `create_signal(0)` → `signal(0)`
- `create_rw_signal(0)` → `signal(0)`
- `create_memo(|| 0)` → `signal(0).derive(|v| 0)`
- `create_resource(|| async { 0 })` → `signal::async(|| (), |_| async { 0 })`
```

### 2. Compatibility Matrix

```markdown
# Compatibility Matrix

| Old API | New API | Compatibility | Migration |
|---------|---------|---------------|-----------|
| `create_signal` | `signal` | ✅ Direct | Drop-in replacement |
| `create_rw_signal` | `signal` | ✅ Direct | Drop-in replacement |
| `create_memo` | `signal().derive()` | ✅ Direct | Method chaining |
| `create_resource` | `signal::async` | ✅ Direct | Function change |
```

### 3. FAQ

```markdown
# Frequently Asked Questions

## Q: Will my existing code break?
A: No, all existing APIs remain functional. The new API is additive.

## Q: How do I migrate my code?
A: Use the automated migration tool: `leptos-migrate`

## Q: What's the performance impact?
A: The new API maintains or improves performance through optimizations.

## Q: When will old APIs be removed?
A: Old APIs will be deprecated in v0.10.0 and removed in v1.0.0.
```

## Community Support

### 1. Migration Assistance

- **Discord**: Real-time help with migration
- **GitHub**: Issue tracking for migration problems
- **Documentation**: Comprehensive migration guides
- **Examples**: Working migration examples

### 2. Community Tools

- **Migration Scripts**: Community-contributed migration tools
- **Validation Tools**: Community validation and testing tools
- **Examples**: Community migration examples
- **Best Practices**: Community migration best practices

### 3. Support Channels

- **Discord**: #migration-help channel
- **GitHub**: Migration issue labels
- **Documentation**: Migration FAQ and guides
- **Community**: Migration success stories

## Conclusion

The backward compatibility plan ensures a smooth transition from the current signal APIs to the new unified API. By maintaining full backward compatibility, providing automated migration tools, and following a clear deprecation timeline, we can ensure that existing code continues to work while encouraging adoption of the new API.

The key to success will be providing excellent migration tools, comprehensive documentation, and strong community support throughout the transition process.

---

**Version**: 1.0  
**Last Updated**: 2025-01-27  
**Status**: Draft for Review  
**Next Steps**: Implementation, testing, community feedback
