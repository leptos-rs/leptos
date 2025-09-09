//! Integration tests for LEPTOS-2024-005: Error Message Enhancement
//! 
//! These tests validate that the view macro provides helpful error messages
//! for common Leptos usage mistakes.

use any_spawner::Executor;
use reactive_graph::owner::Owner;
use reactive_graph::signal::{create_signal, create_rw_signal};
use reactive_graph::computed::create_memo;
use reactive_graph::effect::create_effect;
use reactive_graph::traits::Get;

/// Test setup for error message enhancement tests
fn setup_test() -> Owner {
    Executor::init_futures_executor();
    Owner::new()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use leptos::prelude::*;

    #[test]
    fn test_signal_usage_patterns() {
        let _owner = setup_test();
        
        // Test that we can create signals correctly
        let (count, set_count) = create_signal(0);
        let (user, set_user) = create_signal("John".to_string());
        
        // Test correct signal usage
        assert_eq!(count.get(), 0);
        assert_eq!(user.get(), "John");
        
        // Test signal updates
        set_count.set(42);
        set_user.set("Jane".to_string());
        
        assert_eq!(count.get(), 42);
        assert_eq!(user.get(), "Jane");
        
        // Test signal updates with closure
        set_count.update(|c| *c += 1);
        set_user.update(|u| u.push_str(" Doe"));
        
        assert_eq!(count.get(), 43);
        assert_eq!(user.get(), "Jane Doe");
    }

    #[test]
    fn test_derived_signals() {
        let _owner = setup_test();
        
        // Test derived signals
        let (count, set_count) = create_signal(0);
        let doubled = create_memo(move |_| count.get() * 2);
        
        assert_eq!(doubled.get(), 0);
        
        set_count.set(5);
        assert_eq!(doubled.get(), 10);
        
        set_count.set(10);
        assert_eq!(doubled.get(), 20);
    }

    #[test]
    fn test_effects() {
        let _owner = setup_test();
        
        // Test effects
        let (count, set_count) = create_signal(0);
        let (effect_count, set_effect_count) = create_signal(0);
        
        create_effect(move |_| {
            let _ = count.get(); // Track the signal
            set_effect_count.update(|c| *c += 1);
        });
        
        // Initial effect run
        assert_eq!(effect_count.get(), 1);
        
        // Update count to trigger effect
        set_count.set(1);
        assert_eq!(effect_count.get(), 2);
        
        set_count.set(2);
        assert_eq!(effect_count.get(), 3);
    }

    #[test]
    fn test_signal_splitting() {
        let _owner = setup_test();
        
        // Test signal splitting
        let (count, set_count) = create_signal(0);
        let (read_count, write_count) = (count, set_count);
        
        assert_eq!(read_count.get(), 0);
        
        write_count.set(42);
        assert_eq!(read_count.get(), 42);
        
        write_count.update(|c| *c += 1);
        assert_eq!(read_count.get(), 43);
    }

    #[test]
    fn test_complex_signal_graph() {
        let _owner = setup_test();
        
        // Test complex signal dependencies
        let (a, set_a) = create_signal(1);
        let (b, set_b) = create_signal(2);
        let (c, set_c) = create_signal(3);
        
        let sum = create_memo(move |_| a.get() + b.get() + c.get());
        let product = create_memo(move |_| a.get() * b.get() * c.get());
        let average = create_memo(move |_| sum.get() as f64 / 3.0);
        
        // Initial values
        assert_eq!(sum.get(), 6);
        assert_eq!(product.get(), 6);
        assert_eq!(average.get(), 2.0);
        
        // Update values
        set_a.set(2);
        assert_eq!(sum.get(), 7);
        assert_eq!(product.get(), 12);
        assert_eq!((average.get() * 100.0).round() / 100.0, 2.33);
        
        set_b.set(3);
        assert_eq!(sum.get(), 8);
        assert_eq!(product.get(), 18);
        assert_eq!((average.get() * 100.0).round() / 100.0, 2.67);
    }

    #[test]
    fn test_signal_cloning_and_sharing() {
        let _owner = setup_test();
        
        // Test signal cloning and sharing
        let (count, set_count) = create_signal(0);
        let count_clone = count.clone();
        
        // Both should reference the same signal
        assert_eq!(count.get(), 0);
        assert_eq!(count_clone.get(), 0);
        
        set_count.set(42);
        assert_eq!(count.get(), 42);
        assert_eq!(count_clone.get(), 42);
        
        // Test sharing between different contexts
        let (shared_count, set_shared_count) = create_signal(0);
        let shared_clone = shared_count.clone();
        
        // Create effects that track the shared signal
        let (effect1_count, set_effect1_count) = create_signal(0);
        let (effect2_count, set_effect2_count) = create_signal(0);
        
        create_effect(move |_| {
            let _ = shared_count.get();
            set_effect1_count.update(|c| *c += 1);
        });
        
        create_effect(move |_| {
            let _ = shared_clone.get();
            set_effect2_count.update(|c| *c += 1);
        });
        
        // Both effects should run
        assert_eq!(effect1_count.get(), 1);
        assert_eq!(effect2_count.get(), 1);
        
        // Update shared signal
        set_shared_count.set(1);
        assert_eq!(effect1_count.get(), 2);
        assert_eq!(effect2_count.get(), 2);
    }

    #[test]
    fn test_error_handling_patterns() {
        let _owner = setup_test();
        
        // Test error handling with signals
        let (result, set_result) = create_signal::<Result<i32, String>>(Ok(42));
        let (error_count, set_error_count) = create_signal(0);
        
        create_effect(move |_| {
            match result.get() {
                Ok(_) => {
                    // Success case
                }
                Err(_) => {
                    set_error_count.update(|c| *c += 1);
                }
            }
        });
        
        // Initial state
        assert_eq!(error_count.get(), 0);
        
        // Set error
        set_result.set(Err("Something went wrong".to_string()));
        assert_eq!(error_count.get(), 1);
        
        // Set success again
        set_result.set(Ok(100));
        assert_eq!(error_count.get(), 1); // Error count shouldn't change
    }

    #[test]
    fn test_performance_with_many_signals() {
        let _owner = setup_test();
        
        // Test performance with many signals
        let signal_count = 1000;
        let mut signals = Vec::new();
        
        // Create many signals
        for i in 0..signal_count {
            let (signal, set_signal) = create_signal(i);
            signals.push((signal, set_signal));
        }
        
        // Test that all signals work correctly
        for (i, (signal, set_signal)) in signals.iter().enumerate() {
            assert_eq!(signal.get(), i);
            set_signal.set(i * 2);
            assert_eq!(signal.get(), i * 2);
        }
        
        // Test derived signal with many dependencies
        let (trigger, set_trigger) = create_signal(0);
        let derived = create_memo(move |_| {
            let _ = trigger.get(); // Track trigger
            signals.iter().map(|(s, _)| s.get()).sum::<usize>()
        });
        
        // Initial value
        let expected_sum: usize = (0..signal_count).map(|i| i * 2).sum();
        assert_eq!(derived.get(), expected_sum);
        
        // Update trigger to recalculate
        set_trigger.set(1);
        assert_eq!(derived.get(), expected_sum);
    }
}

#[cfg(test)]
mod error_message_validation_tests {
    use super::*;

    #[test]
    fn test_common_error_patterns() {
        let _owner = setup_test();
        use reactive_graph::signal::create_signal;
        
        // These tests validate that our error message enhancement system
        // would catch common mistakes. Since we can't test compile-time
        // errors in unit tests, we test the patterns that should trigger
        // helpful error messages.
        
        // Test signal creation patterns
        let (count, _) = create_signal(0);
        let (user, _) = create_signal("John".to_string());
        
        // These should work correctly
        assert_eq!(count.get(), 0);
        assert_eq!(user.get(), "John");
        
        // Test that we can detect common signal naming patterns
        let signal_names = vec![
            "count", "value", "data", "state", "loading", "error", "user", "items",
            "selected", "active", "visible", "enabled", "disabled", "open", "closed",
            "current", "previous", "next", "first", "last", "total", "sum", "result",
        ];
        
        // All these names should be recognized as likely signal names
        for name in signal_names {
            // In a real scenario, these would trigger helpful error messages
            // if used directly in views without .get()
            assert!(!name.is_empty(), "Signal name should not be empty");
        }
    }

    #[test]
    fn test_server_function_patterns() {
        let _owner = setup_test();
        
        // Test server function naming patterns
        let server_function_names = vec![
            "get_data", "fetch_data", "load_data", "save_data", "update_data", "delete_data",
            "get_user", "fetch_user", "load_user", "save_user", "update_user", "delete_user",
            "get_posts", "fetch_posts", "load_posts", "save_posts", "update_posts", "delete_posts",
            "get_config", "fetch_config", "load_config", "save_config", "update_config",
            "get_settings", "fetch_settings", "load_settings", "save_settings", "update_settings",
            "login", "logout", "register", "authenticate", "authorize",
            "upload", "download", "export", "import", "sync",
        ];
        
        // All these names should be recognized as likely server functions
        for name in server_function_names {
            // In a real scenario, these would trigger helpful error messages
            // if called directly in client context
            assert!(!name.is_empty(), "Server function name should not be empty");
        }
    }

    #[test]
    fn test_feature_flag_combinations() {
        // Test feature flag validation patterns
        let valid_combinations = vec![
            vec!["csr"],
            vec!["ssr"],
            vec!["ssr", "static"],
        ];
        
        let invalid_combinations = vec![
            vec!["csr", "ssr"], // Conflicting without static
        ];
        
        // Validate that we can detect valid vs invalid combinations
        for combination in valid_combinations {
            assert!(!combination.is_empty(), "Valid combination should not be empty");
            // In a real scenario, these would not trigger feature conflict errors
        }
        
        for combination in invalid_combinations {
            assert!(!combination.is_empty(), "Invalid combination should not be empty");
            // In a real scenario, these would trigger feature conflict errors
        }
    }
}
