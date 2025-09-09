# Unified Signal API - User Stories & Use Cases

## Overview

This document outlines user stories and use cases for the unified signal API, covering different developer personas and scenarios to ensure the API meets real-world needs.

## User Personas

### 1. **Beginner Developer (Sarah)**
- New to Rust and Leptos
- Coming from React/Vue background
- Wants to build simple interactive apps
- Needs clear, consistent APIs

### 2. **Experienced Developer (Alex)**
- Familiar with Rust and web development
- Building complex applications
- Needs performance and flexibility
- Wants to optimize for specific use cases

### 3. **Framework Maintainer (Jordan)**
- Contributing to Leptos ecosystem
- Building libraries and tools
- Needs stable, extensible APIs
- Wants to provide good developer experience

### 4. **Migrating Developer (Casey)**
- Has existing Leptos codebase
- Needs smooth migration path
- Wants to maintain performance
- Needs clear migration guidance

## User Stories

### Beginner Stories

#### Story 1: Simple Counter
**As a beginner developer**, I want to create a simple counter so that I can understand basic reactivity without confusion.

```rust
// Current (confusing)
let (count, set_count) = create_signal(0);
// or
let count = create_rw_signal(0);

// Desired (unified)
let count = signal(0);

// Usage
view! {
    <button on:click=move |_| count.update(|c| *c += 1)>
        "Count: " {count.get()}
    </button>
}
```

**Acceptance Criteria:**
- One clear way to create a signal
- Consistent method names (.get(), .set(), .update())
- Works in view! macro without special syntax

#### Story 2: Form Input
**As a beginner developer**, I want to bind form inputs to signals so that I can create interactive forms easily.

```rust
let name = signal(String::new());
let email = signal(String::new());

view! {
    <input
        type="text"
        prop:value=name.get()
        on:input=move |ev| name.set(event_target_value(&ev))
    />
    <input
        type="email"
        prop:value=email.get()
        on:input=move |ev| email.set(event_target_value(&ev))
    />
}
```

**Acceptance Criteria:**
- Simple two-way binding
- No need to understand read/write signal splitting
- Works with standard HTML events

#### Story 3: Derived Values
**As a beginner developer**, I want to create computed values so that I can display calculated data without manual updates.

```rust
let price = signal(100.0);
let tax_rate = signal(0.08);

// Simple derivation
let tax = price.derive(|p| *p * tax_rate.get());
let total = price.derive(|p| *p + tax.get());

view! {
    <div>
        <p>"Price: $" {price.get()}</p>
        <p>"Tax: $" {tax.get()}</p>
        <p>"Total: $" {total.get()}</p>
    </div>
}
```

**Acceptance Criteria:**
- Simple .derive() method
- Automatic reactivity
- No need to understand Memo vs other types

### Experienced Developer Stories

#### Story 4: Performance Optimization
**As an experienced developer**, I want to split signals for read-only props so that I can optimize re-renders.

```rust
let user_data = signal(UserData::new());

// Split for passing read-only to children
let (user_read, user_write) = user_data.split();

// Pass read-only to child components
view! {
    <UserProfile user=user_read />
    <UserSettings on:update=move |data| user_write.set(data) />
}
```

**Acceptance Criteria:**
- Explicit control over read/write access
- Performance benefits maintained
- Clear API for splitting

#### Story 5: Complex Async Data
**As an experienced developer**, I want to handle complex async data flows so that I can build robust applications.

```rust
let user_id = signal(1);
let search_query = signal(String::new());

// Async signal with dependencies
let users = signal::async(
    move || (user_id.get(), search_query.get()),
    |(id, query)| async move {
        if query.is_empty() {
            fetch_all_users().await
        } else {
            search_users(id, &query).await
        }
    }
);

// Error handling
let posts = signal::async_with_error(
    move || user_id.get(),
    |id| async { fetch_user_posts(id).await }
);
```

**Acceptance Criteria:**
- Flexible async signal creation
- Error handling built-in
- Dependency tracking
- Integration with Suspense

#### Story 6: Custom Signal Types
**As an experienced developer**, I want to create custom signal types so that I can extend the system for specific needs.

```rust
// Custom signal for local storage
struct LocalStorageSignal<T> {
    key: String,
    signal: RwSignal<T>,
}

impl<T: Clone + Serialize + DeserializeOwned + 'static> Signal<T> for LocalStorageSignal<T> {
    fn get(&self) -> T {
        // Load from localStorage
        self.signal.get()
    }
    
    fn set(&self, value: T) {
        // Save to localStorage
        self.signal.set(value);
    }
    
    // ... other trait methods
}

// Usage
let settings = signal::local("app-settings", default_settings);
```

**Acceptance Criteria:**
- Extensible trait system
- Custom implementations possible
- Integration with unified API

### Framework Maintainer Stories

#### Story 7: Library Integration
**As a framework maintainer**, I want to provide signals in my library so that users get a consistent experience.

```rust
// In a form library
pub struct FormSignal<T> {
    value: impl Signal<T>,
    errors: impl Signal<Vec<String>>,
    touched: impl Signal<bool>,
}

impl<T> FormSignal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: signal(initial),
            errors: signal(Vec::new()),
            touched: signal(false),
        }
    }
    
    pub fn value(&self) -> impl Signal<T> {
        self.value.clone()
    }
    
    pub fn errors(&self) -> impl Signal<Vec<String>> {
        self.errors.clone()
    }
}
```

**Acceptance Criteria:**
- Easy to create library-specific signals
- Consistent with core API
- Good for composition

#### Story 8: Testing Utilities
**As a framework maintainer**, I want to provide testing utilities so that users can test reactive code easily.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signal_updates() {
        let count = signal(0);
        
        // Test initial value
        assert_eq!(count.get(), 0);
        
        // Test updates
        count.set(42);
        assert_eq!(count.get(), 42);
        
        // Test functional updates
        count.update(|c| *c += 1);
        assert_eq!(count.get(), 43);
    }
    
    #[test]
    fn test_derived_signals() {
        let base = signal(10);
        let doubled = base.derive(|b| *b * 2);
        
        assert_eq!(doubled.get(), 20);
        
        base.set(5);
        assert_eq!(doubled.get(), 10);
    }
}
```

**Acceptance Criteria:**
- Easy to test signal behavior
- Clear testing patterns
- Good error messages

### Migration Stories

#### Story 9: Gradual Migration
**As a migrating developer**, I want to gradually migrate my existing code so that I can adopt the new API incrementally.

```rust
// Old code
let (count, set_count) = create_signal(0);
let doubled = create_memo(move || count.get() * 2);

// New code (drop-in replacement)
let count = signal(0);
let doubled = count.derive(|c| *c * 2);

// Mixed code (during migration)
let old_signal = create_signal(0);
let new_signal = signal(0);

// Both work together
let combined = signal::computed(|| {
    old_signal.get() + new_signal.get()
});
```

**Acceptance Criteria:**
- Drop-in replacements available
- Old and new APIs work together
- Clear migration path
- No breaking changes

#### Story 10: Performance Validation
**As a migrating developer**, I want to validate that the new API doesn't hurt performance so that I can migrate with confidence.

```rust
// Benchmark comparison
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn bench_old_signal(c: &mut Criterion) {
        c.bench_function("old_signal", |b| {
            let (count, set_count) = create_signal(0);
            b.iter(|| {
                set_count.set(black_box(count.get() + 1));
            });
        });
    }
    
    fn bench_new_signal(c: &mut Criterion) {
        c.bench_function("new_signal", |b| {
            let count = signal(0);
            b.iter(|| {
                count.update(|c| *c = black_box(*c + 1));
            });
        });
    }
    
    criterion_group!(benches, bench_old_signal, bench_new_signal);
    criterion_main!(benches);
}
```

**Acceptance Criteria:**
- Performance benchmarks available
- Migration doesn't hurt performance
- Clear performance guidelines

## Use Case Scenarios

### Scenario 1: Todo Application

```rust
#[derive(Clone)]
struct Todo {
    id: u32,
    text: String,
    completed: bool,
}

fn TodoApp() -> impl IntoView {
    let todos = signal(Vec::<Todo>::new());
    let new_todo = signal(String::new());
    let filter = signal("all".to_string());
    
    // Derived signals
    let filtered_todos = todos.derive(|todos| {
        match filter.get().as_str() {
            "active" => todos.iter().filter(|t| !t.completed).cloned().collect(),
            "completed" => todos.iter().filter(|t| t.completed).cloned().collect(),
            _ => todos.clone(),
        }
    });
    
    let remaining_count = todos.derive(|todos| {
        todos.iter().filter(|t| !t.completed).count()
    });
    
    let add_todo = move |_| {
        let text = new_todo.get();
        if !text.is_empty() {
            todos.update(|todos| {
                todos.push(Todo {
                    id: todos.len() as u32,
                    text,
                    completed: false,
                });
            });
            new_todo.set(String::new());
        }
    };
    
    view! {
        <div>
            <input
                prop:value=new_todo.get()
                on:input=move |ev| new_todo.set(event_target_value(&ev))
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        add_todo(());
                    }
                }
            />
            <button on:click=add_todo>"Add"</button>
            
            <div>
                <button on:click=move |_| filter.set("all".to_string())>"All"</button>
                <button on:click=move |_| filter.set("active".to_string())>"Active"</button>
                <button on:click=move |_| filter.set("completed".to_string())>"Completed"</button>
            </div>
            
            <p>"Remaining: " {remaining_count.get()}</p>
            
            <For
                each=move || filtered_todos.get()
                key=|todo| todo.id
                children=move |todo| {
                    let todos = todos.clone();
                    view! {
                        <div>
                            <input
                                type="checkbox"
                                prop:checked=todo.completed
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    todos.update(|todos| {
                                        if let Some(t) = todos.iter_mut().find(|t| t.id == todo.id) {
                                            t.completed = checked;
                                        }
                                    });
                                }
                            />
                            <span>{todo.text}</span>
                        </div>
                    }
                }
            />
        </div>
    }
}
```

### Scenario 2: Data Dashboard

```rust
fn Dashboard() -> impl IntoView {
    let selected_user = signal(1);
    let date_range = signal((chrono::Utc::now() - chrono::Duration::days(30), chrono::Utc::now()));
    
    // Async signals for data fetching
    let user = signal::async(
        move || selected_user.get(),
        |id| async { fetch_user(id).await }
    );
    
    let stats = signal::async(
        move || (selected_user.get(), date_range.get()),
        |(user_id, (start, end))| async {
            fetch_user_stats(user_id, start, end).await
        }
    );
    
    let charts = signal::async(
        move || (selected_user.get(), date_range.get()),
        |(user_id, (start, end))| async {
            fetch_chart_data(user_id, start, end).await
        }
    );
    
    view! {
        <div>
            <UserSelector selected=selected_user />
            <DateRangePicker range=date_range />
            
            <Suspense fallback=move || view! { <div>"Loading..."</div> }>
                <UserProfile user=user />
                <StatsDisplay stats=stats />
                <ChartsDisplay charts=charts />
            </Suspense>
        </div>
    }
}
```

### Scenario 3: Form with Validation

```rust
#[derive(Clone)]
struct FormData {
    name: String,
    email: String,
    age: u32,
}

fn ContactForm() -> impl IntoView {
    let form_data = signal(FormData {
        name: String::new(),
        email: String::new(),
        age: 0,
    });
    
    // Validation signals
    let name_error = form_data.derive(|data| {
        if data.name.is_empty() {
            "Name is required".to_string()
        } else if data.name.len() < 2 {
            "Name must be at least 2 characters".to_string()
        } else {
            String::new()
        }
    });
    
    let email_error = form_data.derive(|data| {
        if data.email.is_empty() {
            "Email is required".to_string()
        } else if !data.email.contains('@') {
            "Invalid email format".to_string()
        } else {
            String::new()
        }
    });
    
    let is_valid = signal::computed(|| {
        name_error.get().is_empty() && email_error.get().is_empty()
    });
    
    let submit = move |_| {
        if is_valid.get() {
            // Submit form
            println!("Submitting: {:?}", form_data.get());
        }
    };
    
    view! {
        <form on:submit=move |ev| { ev.prevent_default(); submit(()); }>
            <div>
                <input
                    type="text"
                    placeholder="Name"
                    prop:value=form_data.get().name
                    on:input=move |ev| {
                        form_data.update(|data| data.name = event_target_value(&ev));
                    }
                />
                <span class="error">{name_error.get()}</span>
            </div>
            
            <div>
                <input
                    type="email"
                    placeholder="Email"
                    prop:value=form_data.get().email
                    on:input=move |ev| {
                        form_data.update(|data| data.email = event_target_value(&ev));
                    }
                />
                <span class="error">{email_error.get()}</span>
            </div>
            
            <div>
                <input
                    type="number"
                    placeholder="Age"
                    prop:value=form_data.get().age
                    on:input=move |ev| {
                        if let Ok(age) = event_target_value(&ev).parse::<u32>() {
                            form_data.update(|data| data.age = age);
                        }
                    }
                />
            </div>
            
            <button type="submit" disabled=move || !is_valid.get()>
                "Submit"
            </button>
        </form>
    }
}
```

## Success Criteria

### Beginner Developer Success
- Can create a simple interactive app in <30 minutes
- Understands signal concepts without confusion
- Can find examples that match their use case

### Experienced Developer Success
- Can optimize performance when needed
- Can extend the system for custom needs
- Maintains existing performance characteristics

### Framework Maintainer Success
- Can build libraries that integrate well
- Can provide good testing utilities
- Can contribute to the ecosystem easily

### Migration Success
- Can migrate existing code incrementally
- No performance regressions
- Clear migration path and tools

## Conclusion

These user stories and use cases provide a comprehensive view of how the unified signal API will be used in practice. They ensure that the API design meets real-world needs while maintaining Leptos's strengths in performance and developer experience.

---

**Version**: 1.0  
**Last Updated**: 2025-01-27  
**Status**: Draft for Review  
**Next Steps**: Community feedback, implementation planning
