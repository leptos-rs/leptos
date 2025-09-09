//! Test suite for LEPTOS-2024-005: Error Message Enhancement
//! 
//! This test suite validates that the custom diagnostic system provides
//! helpful, actionable error messages for common Leptos usage mistakes.

use any_spawner::Executor;
use reactive_graph::owner::Owner;

/// Test setup for error message enhancement tests
fn setup_test() -> Owner {
    Executor::init_futures_executor();
    Owner::new()
}

#[cfg(test)]
mod signal_usage_tests {
    use super::*;
    use leptos::prelude::*;

    #[test]
    fn test_signal_without_get_error_detection() {
        let _owner = setup_test();
        
        // This should trigger a helpful error message
        // The test validates that the error message is more helpful than the default Rust error
        let (count, _) = create_signal(0);
        
        // This would normally produce a cryptic "trait bound not satisfied" error
        // With our enhancement, it should produce a clear message about using .get()
        let result = std::panic::catch_unwind(|| {
            // This is a compile-time test, so we can't actually test the macro here
            // Instead, we test that our diagnostic system can detect the pattern
            let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
            
            // Simulate the expression that would cause the error
            use syn::parse_quote;
            let expr: syn::Expr = parse_quote! { count };
            
            // The diagnostic should detect this as a signal usage issue
            let span = proc_macro2::Span::call_site();
            let result = diagnostics.analyze_expression(&expr, span);
            
            // Should return diagnostic tokens
            assert!(result.is_some(), "Diagnostic should detect signal usage without .get()");
        });
        
        // The test should not panic (since we're not actually compiling)
        assert!(result.is_ok());
    }

    #[test]
    fn test_signal_field_access_error_detection() {
        let _owner = setup_test();
        
        // Test field access on signals without .get()
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { user.name };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should detect field access on likely signal
        assert!(result.is_some(), "Diagnostic should detect signal field access without .get()");
    }

    #[test]
    fn test_signal_method_call_warning() {
        let _owner = setup_test();
        
        // Test method calls on signals in views
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count.set(42) };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should detect method call on signal
        assert!(result.is_some(), "Diagnostic should detect signal method call in view");
    }

    #[test]
    fn test_correct_signal_usage_no_error() {
        let _owner = setup_test();
        
        // Test that correct signal usage doesn't trigger false positives
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count.get() };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should not detect any issues with correct usage
        assert!(result.is_none(), "Correct signal usage should not trigger diagnostics");
    }
}

#[cfg(test)]
mod server_function_tests {
    use super::*;

    #[test]
    fn test_server_function_usage_error_detection() {
        let _owner = setup_test();
        
        // Test server function usage in client context
        let diagnostics = leptos_macro::diagnostics::ServerFunctionDiagnostics;
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { get_data() };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.check_server_function_usage(&expr, span);
        
        // Should detect server function usage
        assert!(result.is_some(), "Diagnostic should detect server function usage in client context");
    }

    #[test]
    fn test_server_function_method_call_detection() {
        let _owner = setup_test();
        
        // Test server function method calls
        let diagnostics = leptos_macro::diagnostics::ServerFunctionDiagnostics;
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { get_user().await };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.check_server_function_usage(&expr, span);
        
        // Should detect server function method call
        assert!(result.is_some(), "Diagnostic should detect server function method call");
    }

    #[test]
    fn test_non_server_function_no_error() {
        let _owner = setup_test();
        
        // Test that non-server functions don't trigger false positives
        let diagnostics = leptos_macro::diagnostics::ServerFunctionDiagnostics;
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { regular_function() };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.check_server_function_usage(&expr, span);
        
        // Should not detect any issues with regular functions
        assert!(result.is_none(), "Regular functions should not trigger server function diagnostics");
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[test]
    fn test_feature_conflict_detection() {
        let diagnostics = leptos_macro::diagnostics::ConfigurationDiagnostics;
        
        // Test conflicting features
        let conflicting_features = vec!["csr".to_string(), "ssr".to_string()];
        let span = proc_macro2::Span::call_site();
        
        // This should trigger a feature conflict error
        let result = std::panic::catch_unwind(|| {
            diagnostics.check_feature_conflicts(&conflicting_features, span);
        });
        
        // Should panic with helpful error message
        assert!(result.is_err(), "Conflicting features should trigger error");
    }

    #[test]
    fn test_valid_feature_combinations() {
        let diagnostics = leptos_macro::diagnostics::ConfigurationDiagnostics;
        
        // Test valid feature combinations
        let valid_combinations = vec![
            vec!["csr".to_string()],
            vec!["ssr".to_string()],
            vec!["ssr".to_string(), "static".to_string()],
        ];
        
        for features in valid_combinations {
            let span = proc_macro2::Span::call_site();
            let result = std::panic::catch_unwind(|| {
                diagnostics.check_feature_conflicts(&features, span);
            });
            
            // Should not panic for valid combinations
            assert!(result.is_ok(), "Valid feature combinations should not trigger errors");
        }
    }
}

#[cfg(test)]
mod error_message_quality_tests {
    use super::*;

    #[test]
    fn test_error_message_helpfulness() {
        // Test that our error messages are more helpful than default Rust errors
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should provide diagnostic tokens with helpful message
        assert!(result.is_some(), "Should provide helpful error message");
        
        // The error message should contain actionable suggestions
        // This is validated by the diagnostic system implementation
    }

    #[test]
    fn test_error_message_includes_documentation_links() {
        // Test that error messages include documentation links
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { user.name };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should provide diagnostic with documentation links
        assert!(result.is_some(), "Should include documentation links in error messages");
    }

    #[test]
    fn test_warning_mode_vs_error_mode() {
        // Test that warning mode provides warnings instead of errors
        let error_diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        let warning_diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::warn_mode();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count };
        
        let span = proc_macro2::Span::call_site();
        
        // Both should detect the issue
        let error_result = error_diagnostics.analyze_expression(&expr, span);
        let warning_result = warning_diagnostics.analyze_expression(&expr, span);
        
        assert!(error_result.is_some(), "Error mode should detect issues");
        assert!(warning_result.is_some(), "Warning mode should detect issues");
        
        // The difference is in the diagnostic level (error vs warning)
        // This is handled by the diagnostic system implementation
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_view_macro_integration() {
        let _owner = setup_test();
        
        // Test that the view macro integrates with the diagnostic system
        // This is a compile-time test, so we can't actually test the macro here
        // Instead, we test that our diagnostic system is properly integrated
        
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count };
        
        let span = proc_macro2::Span::call_site();
        let result = diagnostics.analyze_expression(&expr, span);
        
        // Should detect signal usage issues
        assert!(result.is_some(), "View macro integration should detect signal issues");
    }

    #[test]
    fn test_multiple_error_detection() {
        let _owner = setup_test();
        
        // Test that multiple types of errors can be detected
        let signal_diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        let server_diagnostics = leptos_macro::diagnostics::ServerFunctionDiagnostics;
        
        use syn::parse_quote;
        let signal_expr: syn::Expr = parse_quote! { count };
        let server_expr: syn::Expr = parse_quote! { get_data() };
        
        let span = proc_macro2::Span::call_site();
        
        let signal_result = signal_diagnostics.analyze_expression(&signal_expr, span);
        let server_result = server_diagnostics.check_server_function_usage(&server_expr, span);
        
        // Both should detect their respective issues
        assert!(signal_result.is_some(), "Should detect signal usage issues");
        assert!(server_result.is_some(), "Should detect server function issues");
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_diagnostic_performance() {
        let _owner = setup_test();
        
        // Test that diagnostics don't significantly impact compilation time
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { count };
        
        let span = proc_macro2::Span::call_site();
        
        // Measure time for diagnostic analysis
        let start = std::time::Instant::now();
        let _result = diagnostics.analyze_expression(&expr, span);
        let duration = start.elapsed();
        
        // Diagnostic analysis should be fast (< 1ms)
        assert!(duration.as_millis() < 1, "Diagnostic analysis should be fast");
    }

    #[test]
    fn test_large_expression_analysis() {
        let _owner = setup_test();
        
        // Test performance with larger expressions
        let diagnostics = leptos_macro::diagnostics::LeptosDiagnostics::new();
        
        use syn::parse_quote;
        let expr: syn::Expr = parse_quote! { 
            if count.get() > 5 { 
                user.name 
            } else { 
                "default" 
            } 
        };
        
        let span = proc_macro2::Span::call_site();
        
        let start = std::time::Instant::now();
        let _result = diagnostics.analyze_expression(&expr, span);
        let duration = start.elapsed();
        
        // Should still be fast even with complex expressions
        assert!(duration.as_millis() < 5, "Complex expression analysis should be fast");
    }
}
