# Unified Signal API - Migration Guide

## Overview

This guide provides step-by-step instructions for migrating from the current Leptos signal APIs to the new unified signal API. The migration is designed to be gradual and non-breaking, allowing you to adopt the new API incrementally.

## Migration Timeline

### Phase 1: Additive (v0.9.0)
- ✅ New unified API available alongside existing APIs
- ✅ No deprecation warnings
- ✅ Full backward compatibility
- ✅ New projects can use unified API immediately

### Phase 2: Deprecation (v0.10.0)
- ⚠️ Deprecation warnings for old APIs
- 🔧 Migration tools available
- 📚 Documentation updated to emphasize unified API
- 🎯 Community examples use unified API

### Phase 3: Removal (v1.0.0)
- 🗑️ Deprecated APIs removed
- 🚀 Complete migration to unified API
- 📖 Legacy documentation archived

## Quick Reference

| Old API | New API | Notes |
|---------|---------|-------|
| `create_signal(0)` | `signal(0)` | Drop-in replacement |
| `create_rw_signal(0)` | `signal(0)` | Drop-in replacement |
| `create_memo(|| count.get() * 2)` | `count.derive(\|c\| *c * 2)` | More intuitive |
| `create_resource(|| async { ... })` | `signal::async(|| (), \|_\| async { ... })` | Consistent API |
| `count.get()` | `count.get()` | No change |
| `set_count(42)` | `count.set(42)` | Method instead of separate function |
| `set_count.update(\|c\| *c += 1)` | `count.update(\|c\| *c += 1)` | Method instead of separate function |

## Step-by-Step Migration

### Step 1: Update Signal Creation

#### Before (Old API)
```rust
use leptos::*;

fn MyComponent() -> impl IntoView {
    let (count, set_count) = create_signal(0);
    let name = create_rw_signal(String::new());
    let doubled = create_memo(move || count.get() * 2);
    
    view! {
        <div>
            <p>"Count: " {count.get()}</p>
            <button on:click=move |_| set_count.set(count.get() + 1)>
                "Increment"
            </button>
        </div>
    }
}
```

#### After (New API)
```rust
use leptos::*;

fn MyComponent() -> impl IntoView {
    let count = signal(0);
    let name = signal(String::new());
    let doubled = count.derive(|c| *c * 2);
    
    view! {
        <div>
            <p>"Count: " {count.get()}</p>
            <button on:click=move |_| count.update(|c| *c += 1)>
                "Increment"
            </button>
        </div>
    }
}
```

### Step 2: Update Signal Usage

#### Before (Old API)
```rust
let (count, set_count) = create_signal(0);

// Reading
let value = count.get();

// Writing
set_count.set(42);
set_count.update(|c| *c += 1);

// In views
view! {
    <p>{count.get()}</p>
    <button on:click=move |_| set_count.set(count.get() + 1)>
        "Click me"
    </button>
}
```

#### After (New API)
```rust
let count = signal(0);

// Reading (no change)
let value = count.get();

// Writing (method instead of separate function)
count.set(42);
count.update(|c| *c += 1);

// In views (no change)
view! {
    <p>{count.get()}</p>
    <button on:click=move |_| count.update(|c| *c += 1)>
        "Click me"
    </button>
}
```

### Step 3: Update Derived Signals

#### Before (Old API)
```rust
let (count, set_count) = create_signal(0);
let doubled = create_memo(move || count.get() * 2);
let is_even = create_memo(move || count.get() % 2 == 0);
```

#### After (New API)
```rust
let count = signal(0);
let doubled = count.derive(|c| *c * 2);
let is_even = count.derive(|c| *c % 2 == 0);
```

### Step 4: Update Async Signals (Resources)

#### Before (Old API)
```rust
let user_id = create_signal(1);
let user = create_resource(
    move || user_id.get(),
    |id| async move { fetch_user(id).await }
);
```

#### After (New API)
```rust
let user_id = signal(1);
let user = signal::async(
    move || user_id.get(),
    |id| async move { fetch_user(id).await }
);
```

### Step 5: Update Complex Patterns

#### Before (Old API)
```rust
let (todos, set_todos) = create_signal(Vec::<Todo>::new());
let (filter, set_filter) = create_signal("all".to_string());

let filtered_todos = create_memo(move || {
    let todos = todos.get();
    let filter = filter.get();
    
    match filter.as_str() {
        "active" => todos.into_iter().filter(|t| !t.completed).collect(),
        "completed" => todos.into_iter().filter(|t| t.completed).collect(),
        _ => todos,
    }
});

let add_todo = move |text: String| {
    set_todos.update(|todos| {
        todos.push(Todo {
            id: todos.len() as u32,
            text,
            completed: false,
        });
    });
};
```

#### After (New API)
```rust
let todos = signal(Vec::<Todo>::new());
let filter = signal("all".to_string());

let filtered_todos = todos.derive(|todos| {
    match filter.get().as_str() {
        "active" => todos.iter().filter(|t| !t.completed).cloned().collect(),
        "completed" => todos.iter().filter(|t| t.completed).cloned().collect(),
        _ => todos.clone(),
    }
});

let add_todo = move |text: String| {
    todos.update(|todos| {
        todos.push(Todo {
            id: todos.len() as u32,
            text,
            completed: false,
        });
    });
};
```

## Migration Tools

### Automated Migration Script

A migration script will be provided to help automate the conversion:

```bash
# Install the migration tool
cargo install leptos-migrate

# Run migration on your project
leptos-migrate --path ./src --dry-run
leptos-migrate --path ./src --apply
```

### Rust Analyzer Refactorings

The migration tool will provide Rust Analyzer refactorings for common patterns:

1. **Convert create_signal to signal**
2. **Convert create_rw_signal to signal**
3. **Convert create_memo to derive**
4. **Convert create_resource to signal::async**

### Manual Migration Checklist

- [ ] Replace `create_signal` with `signal`
- [ ] Replace `create_rw_signal` with `signal`
- [ ] Replace `create_memo` with `.derive()`
- [ ] Replace `create_resource` with `signal::async`
- [ ] Update setter function calls to method calls
- [ ] Update `update` function calls to method calls
- [ ] Test all signal interactions
- [ ] Verify performance characteristics
- [ ] Update documentation and comments

## Common Migration Patterns

### Pattern 1: Split Signals

#### Before (Old API)
```rust
let (count, set_count) = create_signal(0);

// Pass read-only to child
<ChildComponent count=count />

// Use setter in parent
<button on:click=move |_| set_count.set(count.get() + 1)>
    "Increment"
</button>
```

#### After (New API)
```rust
let count = signal(0);

// Option 1: Pass the whole signal (recommended)
<ChildComponent count=count.clone() />

// Option 2: Split explicitly if needed
let (count_read, count_write) = count.split();
<ChildComponent count=count_read />
<button on:click=move |_| count_write.update(|c| *c += 1)>
    "Increment"
</button>
```

### Pattern 2: Conditional Logic

#### Before (Old API)
```rust
let (user, set_user) = create_signal(None::<User>);
let is_logged_in = create_memo(move || user.get().is_some());
let display_name = create_memo(move || {
    user.get().map(|u| u.name).unwrap_or_else(|| "Guest".to_string())
});
```

#### After (New API)
```rust
let user = signal(None::<User>);
let is_logged_in = user.derive(|u| u.is_some());
let display_name = user.derive(|u| {
    u.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "Guest".to_string())
});
```

### Pattern 3: Form Handling

#### Before (Old API)
```rust
let (form_data, set_form_data) = create_signal(FormData::default());
let (errors, set_errors) = create_signal(Vec::<String>::new());

let is_valid = create_memo(move || {
    errors.get().is_empty() && !form_data.get().name.is_empty()
});

let submit = move |_| {
    if is_valid.get() {
        // Submit form
        submit_form(form_data.get());
    }
};
```

#### After (New API)
```rust
let form_data = signal(FormData::default());
let errors = signal(Vec::<String>::new());

let is_valid = signal::computed(|| {
    errors.get().is_empty() && !form_data.get().name.is_empty()
});

let submit = move |_| {
    if is_valid.get() {
        // Submit form
        submit_form(form_data.get());
    }
};
```

## Performance Considerations

### Benchmarking Your Migration

Before and after migration, benchmark your signal usage:

```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn bench_signal_operations(c: &mut Criterion) {
        c.bench_function("signal_get_set", |b| {
            let signal = signal(0);
            b.iter(|| {
                let value = black_box(signal.get());
                signal.set(black_box(value + 1));
            });
        });
        
        c.bench_function("signal_update", |b| {
            let signal = signal(0);
            b.iter(|| {
                signal.update(|v| *v = black_box(*v + 1));
            });
        });
        
        c.bench_function("derived_signal", |b| {
            let base = signal(0);
            let derived = base.derive(|v| *v * 2);
            b.iter(|| {
                base.set(black_box(derived.get() / 2));
            });
        });
    }
    
    criterion_group!(benches, bench_signal_operations);
    criterion_main!(benches);
}
```

### Performance Targets

- **Signal Creation**: <5% overhead vs old API
- **Signal Access**: <2% overhead vs old API
- **Signal Updates**: <3% overhead vs old API
- **Derived Signals**: <5% overhead vs old API

## Troubleshooting

### Common Issues

#### Issue 1: Type Inference Problems
```rust
// Problem: Type inference fails
let data = signal(vec![]); // Error: cannot infer type

// Solution: Provide explicit type
let data = signal::<Vec<i32>>(vec![]);
// or
let data: Signal<Vec<i32>> = signal(vec![]);
```

#### Issue 2: Borrowing Issues
```rust
// Problem: Cannot borrow signal mutably
let count = signal(0);
let doubled = count.derive(|c| *c * 2);
count.set(42); // Error: cannot borrow as mutable

// Solution: Clone the signal
let count = signal(0);
let doubled = count.clone().derive(|c| *c * 2);
count.set(42); // OK
```

#### Issue 3: Lifetime Issues
```rust
// Problem: Lifetime issues with derived signals
let data = signal(vec![1, 2, 3]);
let filtered = data.derive(|d| d.iter().filter(|&x| *x > 1).collect::<Vec<_>>());

// Solution: Clone the data
let filtered = data.derive(|d| d.iter().filter(|&x| *x > 1).cloned().collect::<Vec<_>>());
```

### Getting Help

- **Documentation**: Check the updated Leptos Book
- **Examples**: Look at the updated examples in the repository
- **Community**: Ask questions in Discord or GitHub Discussions
- **Issues**: Report bugs or migration problems on GitHub

## Testing Your Migration

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signal_basic_operations() {
        let count = signal(0);
        
        // Test initial value
        assert_eq!(count.get(), 0);
        
        // Test set
        count.set(42);
        assert_eq!(count.get(), 42);
        
        // Test update
        count.update(|c| *c += 1);
        assert_eq!(count.get(), 43);
    }
    
    #[test]
    fn test_derived_signal() {
        let base = signal(10);
        let doubled = base.derive(|b| *b * 2);
        
        assert_eq!(doubled.get(), 20);
        
        base.set(5);
        assert_eq!(doubled.get(), 10);
    }
    
    #[test]
    fn test_async_signal() {
        let trigger = signal(0);
        let data = signal::async(
            move || trigger.get(),
            |_| async { "Hello, World!".to_string() }
        );
        
        // Initially loading
        assert_eq!(data.get(), None);
        
        // Trigger the async operation
        trigger.set(1);
        
        // Should eventually have data (in a real test, you'd need to handle async)
        // This is a simplified example
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_component_with_signals() {
        let count = signal(0);
        let doubled = count.derive(|c| *c * 2);
        
        // Test that the component renders correctly
        let view = view! {
            <div>
                <p>"Count: " {count.get()}</p>
                <p>"Doubled: " {doubled.get()}</p>
            </div>
        };
        
        // In a real test, you'd render and check the output
        // This is a simplified example
    }
}
```

## Conclusion

The migration to the unified signal API is designed to be smooth and incremental. The new API provides better developer experience while maintaining performance and backward compatibility. Follow this guide step by step, and don't hesitate to ask for help if you encounter any issues.

---

**Version**: 1.0  
**Last Updated**: 2025-01-27  
**Status**: Draft for Review  
**Next Steps**: Community feedback, tool development, implementation
