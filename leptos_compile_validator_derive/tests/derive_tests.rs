//! Tests for the derive macros

use leptos_compile_validator_derive::*;
use quote::quote;
use syn::parse_quote;

#[test]
fn test_context_aware_derive_server_only() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        #[leptos::server_only]
        struct DatabaseConnection {
            connection_string: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should contain validation code
    assert!(code.to_string().contains("__leptos_validate_context"));
    assert!(code.to_string().contains("not(feature = \"ssr\")"));
}

#[test]
fn test_context_aware_derive_client_only() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        #[leptos::client_only]
        struct WebSocketClient {
            url: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should contain validation code
    assert!(code.to_string().contains("__leptos_validate_context"));
    assert!(code.to_string().contains("not(any(feature = \"csr\", feature = \"hydrate\"))"));
}

#[test]
fn test_context_aware_derive_universal() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        #[leptos::universal]
        struct User {
            id: u32,
            name: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should contain validation code
    assert!(code.to_string().contains("__leptos_validate_context"));
    // Should not contain feature checks for universal types
    assert!(!code.to_string().contains("not(feature"));
}

#[test]
fn test_context_aware_derive_default() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        struct DefaultStruct {
            field: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should default to universal
    assert!(code.to_string().contains("__leptos_validate_context"));
    assert!(!code.to_string().contains("not(feature"));
}

#[test]
fn test_feature_validated_derive() {
    let input = parse_quote! {
        #[derive(FeatureValidated)]
        #[leptos::requires_features("ssr", "tracing")]
        struct ServerComponent {
            data: String,
        }
    };
    
    let result = derive_feature_validated_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should contain validation code
    assert!(code.to_string().contains("__leptos_validate_features"));
    assert!(code.to_string().contains("not(feature = \"ssr\")"));
    assert!(code.to_string().contains("not(feature = \"tracing\")"));
}

#[test]
fn test_feature_validated_derive_no_features() {
    let input = parse_quote! {
        #[derive(FeatureValidated)]
        struct SimpleStruct {
            field: String,
        }
    };
    
    let result = derive_feature_validated_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    
    // Should be empty for no features
    assert!(tokens.is_empty());
}

#[test]
fn test_extract_context_from_attrs() {
    use leptos_compile_validator_derive::*;
    
    // Test server_only
    let attrs = vec![parse_quote!(#[leptos::server_only])];
    let context = extract_context_from_attrs(&attrs).unwrap();
    assert_eq!(context, ContextType::ServerOnly);
    
    // Test client_only
    let attrs = vec![parse_quote!(#[leptos::client_only])];
    let context = extract_context_from_attrs(&attrs).unwrap();
    assert_eq!(context, ContextType::ClientOnly);
    
    // Test universal
    let attrs = vec![parse_quote!(#[leptos::universal])];
    let context = extract_context_from_attrs(&attrs).unwrap();
    assert_eq!(context, ContextType::Universal);
    
    // Test default (no attributes)
    let attrs = vec![];
    let context = extract_context_from_attrs(&attrs).unwrap();
    assert_eq!(context, ContextType::Universal);
}

#[test]
fn test_extract_required_features() {
    use leptos_compile_validator_derive::*;
    
    // Test with features
    let attrs = vec![parse_quote!(#[leptos::requires_features("ssr", "tracing")])];
    let features = extract_required_features(&attrs).unwrap();
    assert_eq!(features, vec!["ssr".to_string(), "tracing".to_string()]);
    
    // Test with single feature
    let attrs = vec![parse_quote!(#[leptos::requires_features("csr")])];
    let features = extract_required_features(&attrs).unwrap();
    assert_eq!(features, vec!["csr".to_string()]);
    
    // Test with no features
    let attrs = vec![];
    let features = extract_required_features(&attrs).unwrap();
    assert!(features.is_empty());
}

#[test]
fn test_server_only_macro() {
    let input = quote! {
        let db = DatabaseConnection::new();
        let result = db.query("SELECT * FROM users").await?;
    };
    
    let result = server_only(input.into());
    
    // Should wrap in feature check
    let code = quote! { #result };
    assert!(code.to_string().contains("feature = \"ssr\""));
    assert!(code.to_string().contains("not(feature = \"ssr\")"));
}

#[test]
fn test_client_only_macro() {
    let input = quote! {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
    };
    
    let result = client_only(input.into());
    
    // Should wrap in feature check
    let code = quote! { #result };
    assert!(code.to_string().contains("any(feature = \"csr\", feature = \"hydrate\")"));
    assert!(code.to_string().contains("not(any(feature = \"csr\", feature = \"hydrate\"))"));
}

#[test]
fn test_mode_conditional_macro() {
    let input = quote! {
        #[mode = "spa"]
        {
            leptos::mount_to_body(App);
        }
        
        #[mode = "fullstack"]
        {
            leptos::mount_to_body_with_context(App, || provide_context(server_data));
        }
    };
    
    let result = mode_conditional(input.into());
    
    // Should generate mode-specific code
    let code = quote! { #result };
    assert!(code.to_string().contains("LEPTOS_MODE"));
    assert!(code.to_string().contains("spa"));
    assert!(code.to_string().contains("fullstack"));
}

#[test]
fn test_server_only_fn_attribute() {
    let input = parse_quote! {
        async fn database_query() -> Result<String, ServerFnError> {
            let db = DatabaseConnection::new();
            Ok("data".to_string())
        }
    };
    
    let result = server_only_fn(proc_macro::TokenStream::new(), input.into());
    
    // Should add validation
    let code = quote! { #result };
    assert!(code.to_string().contains("not(feature = \"ssr\")"));
    assert!(code.to_string().contains("compile_error"));
}

#[test]
fn test_client_only_fn_attribute() {
    let input = parse_quote! {
        fn setup_web_socket() -> Result<(), JsValue> {
            let ws = WebSocket::new("ws://localhost:8080")?;
            Ok(())
        }
    };
    
    let result = client_only_fn(proc_macro::TokenStream::new(), input.into());
    
    // Should add validation
    let code = quote! { #result };
    assert!(code.to_string().contains("not(any(feature = \"csr\", feature = \"hydrate\"))"));
    assert!(code.to_string().contains("compile_error"));
}

#[test]
fn test_complex_derive_combination() {
    let input = parse_quote! {
        #[derive(ContextAware, FeatureValidated)]
        #[leptos::server_only]
        #[leptos::requires_features("ssr", "tracing")]
        struct DatabaseService {
            connection: String,
            pool_size: u32,
        }
    };
    
    let context_result = derive_context_aware_impl(input.clone());
    let feature_result = derive_feature_validated_impl(input);
    
    assert!(context_result.is_ok());
    assert!(feature_result.is_ok());
    
    let context_tokens = context_result.unwrap();
    let feature_tokens = feature_result.unwrap();
    
    let context_code = quote! { #context_tokens };
    let feature_code = quote! { #feature_tokens };
    
    // Should have both validations
    assert!(context_code.to_string().contains("__leptos_validate_context"));
    assert!(feature_code.to_string().contains("__leptos_validate_features"));
}

#[test]
fn test_derive_with_multiple_attributes() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        #[leptos::server_only]
        #[leptos::requires_features("ssr")]
        struct ServerStruct {
            field: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should handle multiple attributes correctly
    assert!(code.to_string().contains("__leptos_validate_context"));
}

#[test]
fn test_derive_with_nested_attributes() {
    let input = parse_quote! {
        #[derive(ContextAware)]
        #[leptos(server_only)]
        struct NestedStruct {
            field: String,
        }
    };
    
    let result = derive_context_aware_impl(input);
    assert!(result.is_ok());
    
    let tokens = result.unwrap();
    let code = quote! { #tokens };
    
    // Should handle nested attributes
    assert!(code.to_string().contains("__leptos_validate_context"));
}
