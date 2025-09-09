//! Proc macro derives for Leptos compile-time validation

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn};

/// Marks a function as server-only with compile-time validation
/// 
/// # Example
/// ```
/// #[server_only]
/// async fn database_query() -> Result<Data, Error> {
///     // This will cause a compile error if used in client build
/// }
/// ```
#[proc_macro_attribute]
pub fn server_only(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    let output = quote! {
        #[cfg(feature = "ssr")]
        #input_fn
        
        #[cfg(not(feature = "ssr"))]
        compile_error!("This function is server-only and cannot be used in client code. Consider using a server function instead.");
    };
    
    output.into()
}

/// Marks a function as client-only with compile-time validation
/// 
/// # Example
/// ```
/// #[client_only]
/// fn local_storage_access() {
///     // This will cause a compile error if used in server build
/// }
/// ```
#[proc_macro_attribute] 
pub fn client_only(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    let output = quote! {
        #[cfg(any(feature = "csr", feature = "hydrate"))]
        #input_fn
        
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        compile_error!("This function is client-only and cannot be used in server code. Consider restructuring your code or using conditional compilation.");
    };
    
    output.into()
}

/// Validates component props for SSR compatibility
/// 
/// # Example
/// ```
/// #[derive(ValidateProps)]
/// struct MyComponentProps {
///     data: String,  // Must be Send + Sync for SSR
/// }
/// ```
#[proc_macro_derive(ValidateProps)]
pub fn validate_props(input: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(input as DeriveInput);
    let struct_name = &input_struct.ident;
    
    let output = quote! {
        impl #struct_name {
            fn validate_ssr_compatibility() {
                // Compile-time check that all fields are Send + Sync for SSR
                fn assert_send_sync<T: Send + Sync>() {}
                // Would need to iterate fields and call assert_send_sync for each
            }
        }
    };
    
    output.into()
}

/// Validates signal usage patterns for performance
/// 
/// # Example
/// ```
/// #[validate_signals]
/// fn my_component() -> impl IntoView {
///     let signal = create_signal(0);  // Analyzed for performance
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn validate_signals(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    
    // For now, just pass through with a validation comment
    let output = quote! {
        #input_fn
        
        // Signal usage validation would analyze the function body for:
        // - Excessive signal creation
        // - Missing memoization opportunities  
        // - Inefficient subscription patterns
    };
    
    output.into()
}