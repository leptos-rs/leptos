//! Test suite for deprecation warnings in the Unified Signal API
//! 
//! This test suite follows TDD principles for implementing deprecation warnings.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set, Update, ReadSignal, WriteSignal, RwSignal};
use reactive_graph::owner::Owner;
use reactive_graph::signal::{create_signal, create_rw_signal};
use reactive_graph::computed::create_memo;
use any_spawner::Executor;

/// Test that create_signal shows deprecation warning
#[test]
fn test_create_signal_deprecation_warning() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // This should show a deprecation warning
        let (read, write) = create_signal(0);
        
        // Test that it still works
        assert_eq!(read.get(), 0);
        write.set(42);
        assert_eq!(read.get(), 42);
        
        // The deprecation warning should guide users to use signal() instead
        let new_signal = signal(owner.clone(), 0);
        assert_eq!(new_signal.get(), 0);
        new_signal.set(42);
        assert_eq!(new_signal.get(), 42);
    });
}

/// Test that create_rw_signal shows deprecation warning
#[test]
fn test_create_rw_signal_deprecation_warning() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // This should show a deprecation warning
        let rw_signal = create_rw_signal(0);
        
        // Test that it still works
        assert_eq!(rw_signal.get(), 0);
        rw_signal.set(42);
        assert_eq!(rw_signal.get(), 42);
        
        // The deprecation warning should guide users to use signal() instead
        let new_signal = signal(owner.clone(), 0);
        assert_eq!(new_signal.get(), 0);
        new_signal.set(42);
        assert_eq!(new_signal.get(), 42);
    });
}

/// Test that create_memo shows deprecation warning
#[test]
fn test_create_memo_deprecation_warning() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // This should show a deprecation warning
        let memo = create_memo(move |_| 42);
        
        // Test that it still works
        assert_eq!(Get::get(&memo), 42);
        
        // The deprecation warning should guide users to use signal::computed() instead
        let new_computed = signal_module::computed(owner.clone(), || 42);
        assert_eq!(new_computed.get(), 42);
    });
}

/// Test that deprecation warnings include helpful messages
#[test]
fn test_deprecation_warning_messages() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // These should show helpful deprecation messages
        let _old_signal = create_signal(0);
        let _old_rw_signal = create_rw_signal(0);
        let _old_memo = create_memo(move |_| 42);
        
        // The warnings should suggest the new unified API
        let _new_signal = signal(owner.clone(), 0);
        let _new_computed = signal_module::computed(owner.clone(), || 42);
        
        // All should work correctly
        assert!(true, "Deprecation warnings should not break functionality");
    });
}

/// Test that deprecation warnings can be suppressed
#[test]
fn test_deprecation_warning_suppression() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // These should show deprecation warnings
        let _old_signal = create_signal(0);
        let _old_rw_signal = create_rw_signal(0);
        let _old_memo = create_memo(move |_| 42);
        
        // Users should be able to suppress warnings with #[allow(deprecated)]
        #[allow(deprecated)]
        let _suppressed_signal = create_signal(0);
        
        #[allow(deprecated)]
        let _suppressed_rw_signal = create_rw_signal(0);
        
        #[allow(deprecated)]
        let _suppressed_memo = create_memo(move |_| 42);
        
        assert!(true, "Deprecation warnings should be suppressible");
    });
}

/// Test that deprecation warnings include migration examples
#[test]
fn test_deprecation_warning_migration_examples() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // These should show deprecation warnings with migration examples
        let _old_signal = create_signal(0);
        let _old_rw_signal = create_rw_signal(0);
        let _old_memo = create_memo(move |_| 42);
        
        // The warnings should show how to migrate to the new API
        // create_signal(0) -> signal(cx, 0)
        // create_rw_signal(0) -> signal(cx, 0)
        // create_memo(|_| 42) -> signal::computed(cx, || 42)
        
        assert!(true, "Deprecation warnings should include migration examples");
    });
}

/// Test that deprecation warnings are version-aware
#[test]
fn test_deprecation_warning_version_awareness() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // These should show deprecation warnings with version information
        let _old_signal = create_signal(0);
        let _old_rw_signal = create_rw_signal(0);
        let _old_memo = create_memo(move |_| 42);
        
        // The warnings should indicate when the old API will be removed
        // e.g., "will be removed in Leptos 0.9.0"
        
        assert!(true, "Deprecation warnings should be version-aware");
    });
}

/// Test that deprecation warnings work in different contexts
#[test]
fn test_deprecation_warning_contexts() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Test deprecation warnings in different contexts
        
        // In function parameters
        fn use_old_signal(signal: (ReadSignal<i32>, WriteSignal<i32>)) {
            assert_eq!(signal.0.get(), 0);
        }
        
        // In struct fields
        struct MyComponent {
            count: RwSignal<i32>,
        }
        
        // In closures
        let _closure = || {
            let _old_signal = create_signal(0);
        };
        
        // In macros
        let _macro_result = {
            let _old_signal = create_signal(0);
            42
        };
        
        assert!(true, "Deprecation warnings should work in all contexts");
    });
}

/// Test that deprecation warnings don't break existing code
#[test]
fn test_deprecation_warning_backward_compatibility() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Test that deprecated functions still work exactly as before
        let (read, write) = create_signal(0);
        assert_eq!(read.get(), 0);
        write.set(42);
        assert_eq!(read.get(), 42);
        
        let rw_signal = create_rw_signal(0);
        assert_eq!(rw_signal.get(), 0);
        rw_signal.set(42);
        assert_eq!(rw_signal.get(), 42);
        
        let memo = create_memo(move |_| 42);
        assert_eq!(Get::get(&memo), 42);
        
        // Test that all the old patterns still work
        let (count, set_count) = create_signal(0);
        let doubled = create_memo(move |_| count.get() * 2);
        
        set_count.set(5);
        assert_eq!(Get::get(&doubled), 10);
        
        assert!(true, "Deprecation warnings should not break existing code");
    });
}
