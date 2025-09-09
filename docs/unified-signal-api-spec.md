# Unified Signal API Specification

## Overview

This document defines the specification for a unified signal API in Leptos, designed to reduce complexity and improve developer experience while maintaining performance and backward compatibility.

## Problem Statement

Leptos currently offers multiple signal types (`create_signal`, `RwSignal`, `Memo`, `Resource`, etc.) leading to:
- **Confusion for Beginners**: Analysis paralysis when choosing signal types
- **Suboptimal Usage**: Developers may choose inefficient types
- **API Inconsistencies**: Varying methods across different signal types
- **High Adoption Barrier**: Complex mental model for new developers

## Solution Overview

Introduce a unified `signal()` function as the primary API entry point, backed by a `Signal` trait that abstracts underlying implementations.

## Core API Design

### 1. Unified Signal Creation

```rust
use leptos::*;

// Basic usage - infers RwSignal-like behavior
let count = signal(0);
let name = signal("Leptos".to_string());

// Reading/writing is consistent
let value = count.get();           // Always .get()
count.set(42);                     // Always .set()
count.update(|v| *v += 1);         // Functional updates
```

### 2. Signal Trait Definition

```rust
pub trait Signal<T: Clone + 'static>: Clone + 'static {
    /// Get the current value
    fn get(&self) -> T;
    
    /// Set a new value
    fn set(&self, value: T);
    
    /// Update the value using a function
    fn update(&self, f: impl FnOnce(&mut T));
    
    /// Create a derived signal
    fn derive<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<U>;
    
    /// Split into read and write signals
    fn split(self) -> (impl ReadSignal<T>, impl WriteSignal<T>);
    
    /// Check if the signal is reactive
    fn is_reactive(&self) -> bool;
    
    /// Get the signal's ID for debugging
    fn id(&self) -> SignalId;
}
```

### 3. Derived and Computed Signals

```rust
// Simple derivation
let doubled = count.derive(|c| *c * 2);

// Standalone computed signal
let computed = signal::computed(|| {
    count.get() + doubled.get()
});

// Conditional derivation
let status = count.derive(|c| {
    if *c > 10 { "high" } else { "low" }
});
```

### 4. Async Signals (Resources)

```rust
// Basic async signal
let user_id = signal(1);
let user = signal::async(
    move || user_id.get(),
    |id| async { fetch_user(id).await }
);

// With error handling
let posts = signal::async_with_error(
    move || user_id.get(),
    |id| async { fetch_posts(id).await }
);

// Manual resource creation
let data = signal::resource(|| async {
    let response = reqwest::get("https://api.example.com/data").await?;
    response.json().await
});
```

### 5. Advanced Features

```rust
// Explicit splitting for read-only props
let (count_read, count_write) = count.split();

// Local storage integration
let settings = signal::local("app-settings", default_settings);

// Server-side only signals
let server_data = signal::server_only(|| get_server_data());

// Client-side only signals
let client_data = signal::client_only(|| get_client_data());
```

## Internal Implementation

### Smart Signal Wrapper

```rust
enum SmartSignal<T> {
    Rw(RwSignal<T>),
    Split(ReadSignal<T>, WriteSignal<T>),
    Memo(Memo<T>),
    Resource(Resource<T>),
    Local(LocalSignal<T>),
    Server(ServerSignal<T>),
}

impl<T: Clone + 'static> Signal<T> for SmartSignal<T> {
    fn get(&self) -> T {
        match self {
            SmartSignal::Rw(s) => s.get(),
            SmartSignal::Split(r, _) => r.get(),
            SmartSignal::Memo(m) => m.get(),
            SmartSignal::Resource(r) => r.get().unwrap_or_default(),
            SmartSignal::Local(l) => l.get(),
            SmartSignal::Server(s) => s.get(),
        }
    }
    
    fn set(&self, value: T) {
        match self {
            SmartSignal::Rw(s) => s.set(value),
            SmartSignal::Split(_, w) => w.set(value),
            SmartSignal::Local(l) => l.set(value),
            _ => panic!("Cannot set read-only signal"),
        }
    }
    
    // ... other trait methods
}
```

### Type Inference and Smart Defaults

```rust
pub fn signal<T: Clone + 'static>(initial: T) -> impl Signal<T> {
    // Smart selection based on type and usage patterns
    match std::any::TypeId::of::<T>() {
        // Special handling for common types
        id if id == std::any::TypeId::of::<i32>() => {
            SmartSignal::Rw(create_rw_signal(initial))
        },
        id if id == std::any::TypeId::of::<String>() => {
            SmartSignal::Rw(create_rw_signal(initial))
        },
        // Default to RwSignal for most cases
        _ => SmartSignal::Rw(create_rw_signal(initial)),
    }
}
```

## Decision Tree

### When to Use Unified Signals

1. **Default Choice**: Use `signal()` for 80% of use cases
2. **Simple State**: Counters, form inputs, UI state
3. **Derived Values**: Computed properties, filtered lists
4. **Async Data**: API calls, server data fetching

### When to Use Legacy APIs

1. **Performance Critical**: When you need specific optimizations
2. **Complex Patterns**: Advanced reactivity patterns
3. **Legacy Code**: Existing codebases (during migration period)

## Edge Cases and Constraints

### Type Constraints

- All signal values must implement `Clone + 'static`
- Async signals return `Option<T>` during loading
- Error signals return `Result<T, E>`

### Borrowing and Lifetimes

```rust
// This works - owned values
let data = signal(vec![1, 2, 3]);

// This works - cloned values
let count = signal(42);
let doubled = count.derive(|c| *c * 2);

// This doesn't work - borrowed values
// let borrowed = signal(&some_reference); // Compile error
```

### Non-Clone Types

For types that don't implement `Clone`, use `Rc<T>` or `Arc<T>`:

```rust
let complex_data = signal(Rc::new(ComplexData::new()));
```

## Performance Considerations

### Zero-Cost Abstractions

- Use generics to avoid trait object overhead
- Compile-time optimization for common patterns
- Inline functions for hot paths

### Memory Usage

- Smart signals use enum variants (no heap allocation for simple cases)
- Lazy evaluation for derived signals
- Efficient cloning strategies

### Benchmarks

Target performance (vs current APIs):
- Creation: <5% overhead
- Read/Write: <2% overhead
- Derivation: <3% overhead

## Migration Strategy

### Phase 1: Additive (v0.9.0)
- Introduce unified API alongside existing APIs
- No deprecation warnings
- Full backward compatibility

### Phase 2: Deprecation (v0.10.0)
- Add deprecation warnings to old APIs
- Provide migration tools
- Update documentation and examples

### Phase 3: Removal (v1.0.0)
- Remove deprecated APIs
- Complete migration to unified API

## Testing Strategy

### Unit Tests
- Signal consistency across all types
- Type inference correctness
- Edge case handling

### Integration Tests
- Macro compatibility
- Server/client synchronization
- Complex signal chains

### Performance Tests
- Benchmark against current APIs
- Memory usage analysis
- Stress testing with large datasets

### User Experience Tests
- Beginner onboarding surveys
- Developer productivity metrics
- Error rate analysis

## Success Metrics

1. **Developer Experience**
   - >90% of beginners use correct signal type on first try
   - <5% increase in support questions about signals
   - 50% reduction in signal-related confusion

2. **Performance**
   - <5% performance regression vs current APIs
   - No memory leaks or significant overhead
   - Maintains Leptos's reactivity advantages

3. **Adoption**
   - 80% of new projects use unified API within 6 months
   - 60% of existing projects migrate within 1 year
   - Positive community feedback (>4.5/5 rating)

## Future Enhancements

### Potential Extensions

1. **Signal Collections**: Arrays and maps of signals
2. **Signal Validation**: Built-in validation and error handling
3. **Signal Persistence**: Automatic serialization/deserialization
4. **Signal Debugging**: Enhanced developer tools and introspection
5. **Signal Testing**: Testing utilities for reactive code

### Integration Opportunities

1. **Form Libraries**: Seamless integration with form handling
2. **State Management**: Global state patterns
3. **Animation**: Reactive animation systems
4. **Routing**: Reactive route parameters

## Conclusion

The unified signal API provides a clear path forward for Leptos, reducing complexity while maintaining power and performance. This specification serves as the foundation for implementation, ensuring consistency and quality throughout the development process.

---

**Version**: 1.0  
**Last Updated**: 2025-01-27  
**Status**: Draft for Review  
**Next Steps**: Community feedback, implementation planning, prototype development
