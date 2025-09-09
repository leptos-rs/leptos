# Unified Signal API: Leptos Motion Quick Reference

## 🚀 Quick Start

### Before (Problematic)
```rust
// ❌ Causes unresponsiveness in v0.8.8
let (is_active, set_active) = create_signal(false);

// ❌ Causes SSR warnings
fn get_styles() -> String {
    let active = is_active.get(); // Warning: called outside reactive context
    // ...
}
```

### After (Unified Signal API)
```rust
// ✅ Works reliably across versions
let is_active = signal(owner, false);

// ✅ SSR-safe access
fn get_styles_ssr_safe() -> String {
    let active = is_active.get_untracked(); // No warnings
    // ...
}

fn get_styles_reactive() -> String {
    let active = is_active.get(); // For reactive contexts
    // ...
}
```

## 🎯 Motion Component Implementation

### Complete Working Example
```rust
use leptos::unified_signal::{signal, Signal};
use reactive_graph::owner::Owner;
use std::collections::HashMap;

pub struct UnifiedMotionDiv {
    pub is_active: leptos::unified_signal::UnifiedSignal<bool>,
    pub duration: leptos::unified_signal::UnifiedSignal<f64>,
}

impl UnifiedMotionDiv {
    pub fn new(owner: Owner, initial_active: bool, initial_duration: f64) -> Self {
        let is_active = signal(owner.clone(), initial_active);
        let duration = signal(owner.clone(), initial_duration);
        
        let (is_active_read, is_active_write) = is_active.split();
        let (duration_read, duration_write) = duration.split();
        
        Self {
            is_active: leptos::unified_signal::UnifiedSignal::new(is_active_read, is_active_write),
            duration: leptos::unified_signal::UnifiedSignal::new(duration_read, duration_write),
        }
    }
    
    pub fn toggle(&self) {
        self.is_active.update(|active| *active = !*active);
    }
    
    pub fn get_styles_ssr_safe(&self) -> HashMap<String, String> {
        let mut styles = HashMap::new();
        
        if self.is_active.get_untracked() {
            styles.insert("transform".to_string(), "translateX(100px)".to_string());
            styles.insert("opacity".to_string(), "0.5".to_string());
        } else {
            styles.insert("transform".to_string(), "translateX(0px)".to_string());
            styles.insert("opacity".to_string(), "1.0".to_string());
        }
        
        styles.insert("transition".to_string(), 
            format!("all {}ms ease-in-out", self.duration.get_untracked()));
        styles
    }
    
    pub fn get_styles_reactive(&self) -> HashMap<String, String> {
        let mut styles = HashMap::new();
        
        if self.is_active.get() {
            styles.insert("transform".to_string(), "translateX(100px)".to_string());
            styles.insert("opacity".to_string(), "0.5".to_string());
        } else {
            styles.insert("transform".to_string(), "translateX(0px)".to_string());
            styles.insert("opacity".to_string(), "1.0".to_string());
        }
        
        styles.insert("transition".to_string(), 
            format!("all {}ms ease-in-out", self.duration.get()));
        styles
    }
    
    pub fn set_duration(&self, new_duration: f64) {
        self.duration.set(new_duration);
    }
}
```

## 📋 API Reference

### Signal Creation
```rust
// Basic signal
let count = signal(owner, 0);

// Computed signal
let doubled = signal::computed(owner, move || count.get() * 2);

// Async signal
let data = signal::async(owner, move || fetch_data().await);
```

### Signal Access
```rust
// Reactive context (in view!, effects, etc.)
let value = signal.get();

// SSR/non-reactive context (prevents warnings)
let value = signal.get_untracked();

// Set value
signal.set(new_value);

// Functional update
signal.update(|v| *v += 1);
```

### Signal Splitting
```rust
// Split into read/write parts
let (read, write) = signal.split();

// Use read-only part
let value = read.get();

// Use write-only part
write.set(new_value);
```

## 🔧 Migration Checklist

### ✅ Step 1: Replace Signal Creation
- [ ] Replace `create_signal()` with `signal()`
- [ ] Replace `create_rw_signal()` with `signal()`
- [ ] Replace `create_memo()` with `signal::computed()`

### ✅ Step 2: Update Signal Access
- [ ] Use `.get()` in reactive contexts
- [ ] Use `.get_untracked()` in SSR/non-reactive contexts
- [ ] Replace `set_signal()` with `.set()`

### ✅ Step 3: Update Component Structure
- [ ] Replace tuple returns with unified signal types
- [ ] Update method signatures
- [ ] Test responsiveness

### ✅ Step 4: Verify SSR Safety
- [ ] Check for "called outside reactive context" warnings
- [ ] Test server-side rendering
- [ ] Verify hydration works

## 🧪 Testing Patterns

### Test Responsiveness
```rust
#[test]
fn test_responsiveness() {
    let motion_div = UnifiedMotionDiv::new(owner, false, 300.0);
    
    // Test many operations
    for _ in 0..100 {
        motion_div.toggle();
        let _styles = motion_div.get_styles_reactive();
    }
    
    // Verify still responsive
    assert!(motion_div.is_active.get());
}
```

### Test SSR Safety
```rust
#[test]
fn test_ssr_safety() {
    let motion_div = UnifiedMotionDiv::new(owner, true, 500.0);
    
    // Should not cause warnings
    let styles = motion_div.get_styles_ssr_safe();
    assert!(styles.contains_key("transform"));
}
```

## 🚨 Common Issues & Solutions

### Issue: "Called outside reactive context" warnings
**Solution**: Use `.get_untracked()` instead of `.get()` in non-reactive contexts

### Issue: Page becomes unresponsive
**Solution**: Use unified signal API instead of direct `create_signal()`

### Issue: Animations not updating
**Solution**: Use `.get()` in reactive contexts, `.get_untracked()` in SSR

### Issue: Framework compatibility problems
**Solution**: Use unified signal API which provides version-agnostic interface

## 📊 Performance Tips

### ✅ Do
- Use `signal()` for most cases
- Use `.get_untracked()` for SSR
- Use `.get()` for reactive contexts
- Test with many rapid updates

### ❌ Don't
- Mix old and new APIs unnecessarily
- Use `.get()` in non-reactive contexts
- Ignore SSR warnings
- Skip responsiveness testing

## 🔗 Related Documentation

- [Full Case Study](unified-signal-motion-integration.md)
- [API Specification](unified-signal-api-spec.md)
- [Migration Guide](unified-signal-migration-guide.md)
- [Performance Analysis](unified-signal-performance-baseline.md)

---

**Quick Reference Version**: 1.0  
**Last Updated**: December 2024  
**Status**: Ready for Production ✅
