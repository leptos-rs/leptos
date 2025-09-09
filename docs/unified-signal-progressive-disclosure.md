# Unified Signal API - Progressive Disclosure Guide

## Overview

This document outlines the progressive disclosure strategy for the unified signal API in Leptos. The goal is to provide a simple, intuitive API for beginners while exposing advanced features and optimizations for experienced developers.

## Progressive Disclosure Principles

### 1. Start Simple
- Provide a single, obvious entry point (`signal()`)
- Hide complexity behind sensible defaults
- Use intuitive method names and patterns

### 2. Reveal Complexity Gradually
- Expose advanced features through method chaining
- Provide specialized APIs for specific use cases
- Maintain backward compatibility with existing patterns

### 3. Maintain Performance
- Zero-cost abstractions where possible
- Compile-time optimizations for common patterns
- Runtime optimizations for complex scenarios

## API Layers

### Layer 1: Basic Signals (Beginner)

#### Simple State Management
```rust
use leptos::*;

// Create a signal with an initial value
let count = signal(0);

// Read the value
let value = count.get();

// Update the value
count.set(42);

// Functional updates
count.update(|c| *c += 1);
```

#### Basic Reactivity
```rust
// Create a derived signal
let doubled = count.derive(|c| *c * 2);

// Use in views
view! {
    <p>"Count: " {count.get()}</p>
    <p>"Doubled: " {doubled.get()}</p>
    <button on:click=move |_| count.update(|c| *c += 1)>
        "Increment"
    </button>
}
```

### Layer 2: Intermediate Patterns (Intermediate)

#### Signal Splitting
```rust
// Split a signal for controlled access
let (count_read, count_write) = count.split();

// Pass read-only access to child components
<ChildComponent count=count_read />

// Use write access in parent
<button on:click=move |_| count_write.update(|c| *c += 1)>
    "Increment"
</button>
```

#### Computed Signals
```rust
// Multi-dependency computations
let a = signal(1);
let b = signal(2);
let sum = signal::computed(|| a.get() + b.get());

// Conditional computations
let is_even = count.derive(|c| *c % 2 == 0);
let message = signal::computed(|| {
    if is_even.get() {
        "Even number"
    } else {
        "Odd number"
    }
});
```

#### Async Signals
```rust
// Simple async data fetching
let user_id = signal(1);
let user = signal::async(
    move || user_id.get(),
    |id| async move { fetch_user(id).await }
);

// Use with Suspense
view! {
    <Suspense fallback=move || view! { <p>"Loading..."</p> }>
        {move || user.get().map(|u| view! { <p>"User: " {u.name}</p> })}
    </Suspense>
}
```

### Layer 3: Advanced Patterns (Expert)

#### Custom Signal Types
```rust
// Non-Clone types
let data = signal::rc(Rc::new(ComplexData::new()));

// Reference-counted signals
let shared = signal::arc(Arc::new(SharedData::new()));

// Weak references
let weak_ref = signal::weak(Weak::new());
```

#### Performance Optimizations
```rust
// Lazy computations
let expensive = signal::lazy(|| {
    // Expensive computation only runs when needed
    compute_expensive_value()
});

// Batched updates
signal::batch(|| {
    a.set(1);
    b.set(2);
    c.set(3);
    // All updates happen atomically
});

// Memoized computations
let memoized = signal::memo(|| {
    // Result is cached until dependencies change
    expensive_computation(a.get(), b.get())
});
```

#### Advanced Reactivity
```rust
// Custom dependency tracking
let custom = signal::custom(|| {
    // Custom reactive logic
    track_dependency(&a);
    track_dependency(&b);
    compute_result()
});

// Effect management
let effect = signal::effect(|| {
    // Side effects that run when dependencies change
    println!("Count changed to: {}", count.get());
});

// Manual invalidation
signal::invalidate(&count);
```

## Learning Path

### Beginner (0-3 months)
1. **Basic Signals**: Learn `signal()`, `.get()`, `.set()`, `.update()`
2. **Simple Reactivity**: Use `.derive()` for computed values
3. **View Integration**: Use signals in `view!` macros
4. **Event Handling**: Connect signals to user interactions

### Intermediate (3-6 months)
1. **Signal Splitting**: Use `.split()` for controlled access
2. **Complex Computations**: Use `signal::computed()` for multi-dependency logic
3. **Async Patterns**: Use `signal::async()` for data fetching
4. **Component Patterns**: Pass signals between components

### Advanced (6+ months)
1. **Performance Optimization**: Use specialized signal types
2. **Custom Reactivity**: Implement custom reactive patterns
3. **Effect Management**: Use effects for side effects
4. **Advanced Patterns**: Implement complex state management

## API Design Patterns

### Method Chaining
```rust
// Fluent API for common patterns
let result = signal(0)
    .derive(|v| *v * 2)
    .derive(|v| *v + 1)
    .derive(|v| *v * 3);
```

### Builder Pattern
```rust
// Configurable signal creation
let signal = signal::builder()
    .initial(0)
    .lazy()
    .memoized()
    .build();
```

### Factory Functions
```rust
// Specialized signal creation
let counter = signal::counter(0);
let toggle = signal::toggle(false);
let list = signal::list(vec![]);
let map = signal::map(HashMap::new());
```

## Documentation Strategy

### Beginner Documentation
- **Focus**: Simple examples and common patterns
- **Style**: Step-by-step tutorials with explanations
- **Examples**: Todo app, counter, form handling
- **Language**: Plain English, minimal jargon

### Intermediate Documentation
- **Focus**: Real-world patterns and best practices
- **Style**: Pattern libraries and case studies
- **Examples**: Data visualization, complex forms, state management
- **Language**: Technical but accessible

### Advanced Documentation
- **Focus**: Performance optimization and advanced patterns
- **Style**: Reference documentation and deep dives
- **Examples**: Custom reactivity, performance tuning, complex architectures
- **Language**: Technical and precise

## Error Messages and Guidance

### Beginner-Friendly Errors
```rust
// Clear, actionable error messages
error[E0277]: the trait bound `String: Clone` is not satisfied
  --> src/main.rs:5:15
   |
5  | let data = signal(complex_data);
   |           ^^^^^^ the trait `Clone` is not implemented for `ComplexData`
   |
   = help: consider using `signal::rc()` for non-Clone types
   = help: or implement `Clone` for `ComplexData`
```

### Progressive Error Messages
```rust
// Start with simple explanation
error[E0277]: cannot derive from non-Clone type
  --> src/main.rs:10:20
   |
10 | let derived = data.derive(|d| d.process());
   |               ^^^^
   |
   = help: the `derive` method requires `Clone` for performance reasons
   = help: consider using `signal::computed()` for complex computations
   = help: or use `signal::rc()` to wrap non-Clone types
```

### Advanced Error Messages
```rust
// Detailed technical information
error[E0277]: lifetime mismatch in signal derivation
  --> src/main.rs:15:25
   |
15 | let derived = base.derive(|b| &b.field);
   |               ^^^^
   |
   = note: the derived signal would have a shorter lifetime than the base signal
   = help: consider cloning the field: `base.derive(|b| b.field.clone())`
   = help: or use `signal::computed()` with explicit lifetime management
```

## Migration and Adoption

### Gradual Adoption
1. **Phase 1**: New projects use unified API
2. **Phase 2**: Existing projects migrate incrementally
3. **Phase 3**: Old APIs deprecated with clear migration paths
4. **Phase 4**: Old APIs removed after sufficient migration time

### Migration Tools
```rust
// Automated migration assistance
leptos-migrate --help

// Interactive migration
leptos-migrate --interactive

// Dry run to see changes
leptos-migrate --dry-run

// Apply changes
leptos-migrate --apply
```

### Community Support
- **Discord**: Real-time help and discussion
- **GitHub**: Issue tracking and feature requests
- **Documentation**: Comprehensive guides and examples
- **Examples**: Working code samples for common patterns

## Performance Considerations

### Zero-Cost Abstractions
```rust
// Compile-time optimization for common patterns
let count = signal(0);
let doubled = count.derive(|c| *c * 2);

// Compiles to efficient code similar to:
// let count = RwSignal::new(0);
// let doubled = Memo::new(move || count.get() * 2);
```

### Runtime Optimizations
```rust
// Smart signal types based on usage patterns
let simple = signal(0);        // Optimized for simple get/set
let derived = signal(0).derive(|v| *v * 2);  // Optimized for derivation
let async = signal::async(|| 0, |_| async { 0 });  // Optimized for async
```

### Memory Efficiency
```rust
// Compact signal representation
struct Signal<T> {
    // Single pointer to optimized implementation
    inner: Box<dyn SignalImpl<T>>,
}

// Specialized implementations for common types
struct SimpleSignal<T> { /* optimized for i32, String, etc. */ }
struct DerivedSignal<T> { /* optimized for derivations */ }
struct AsyncSignal<T> { /* optimized for async operations */ }
```

## Testing and Validation

### Beginner Testing
```rust
#[test]
fn test_basic_signal_operations() {
    let count = signal(0);
    
    assert_eq!(count.get(), 0);
    count.set(42);
    assert_eq!(count.get(), 42);
    count.update(|c| *c += 1);
    assert_eq!(count.get(), 43);
}
```

### Intermediate Testing
```rust
#[test]
fn test_derived_signals() {
    let base = signal(10);
    let derived = base.derive(|b| *b * 2);
    
    assert_eq!(derived.get(), 20);
    base.set(5);
    assert_eq!(derived.get(), 10);
}
```

### Advanced Testing
```rust
#[test]
fn test_performance_characteristics() {
    let signal = signal(0);
    
    // Benchmark signal operations
    let start = std::time::Instant::now();
    for _ in 0..1_000_000 {
        signal.set(signal.get() + 1);
    }
    let duration = start.elapsed();
    
    // Ensure performance is within acceptable limits
    assert!(duration.as_millis() < 100);
}
```

## Conclusion

The progressive disclosure strategy ensures that the unified signal API is accessible to developers of all skill levels while providing the power and flexibility needed for complex applications. By carefully designing the API layers and providing appropriate documentation and tooling, we can create a system that grows with the developer's needs.

---

**Version**: 1.0  
**Last Updated**: 2025-01-27  
**Status**: Draft for Review  
**Next Steps**: Implementation, testing, community feedback
