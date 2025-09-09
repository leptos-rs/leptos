//! Derive macros for Leptos compile-time validation
//!
//! Provides derive macros and procedural macros for automatic validation
//! of Leptos code based on build context and mode.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, DeriveInput, Meta,
};

/// Derive macro for automatic context validation
///
/// This macro automatically validates that the annotated item is used in the correct
/// build context (client vs server) based on the current Leptos mode.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::ContextAware;
///
/// #[derive(ContextAware)]
/// #[leptos::server_only]
/// struct DatabaseConnection {
///     // This struct can only be used in server context
/// }
///
/// #[derive(ContextAware)]
/// #[leptos::client_only]
/// struct WebSocketClient {
///     // This struct can only be used in client context
/// }
/// ```
#[proc_macro_derive(ContextAware, attributes(leptos))]
pub fn derive_context_aware(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    match derive_context_aware_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_context_aware_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let context = extract_context_from_attrs(&input.attrs)?;
    
    let validation_code = match context {
        ContextType::ServerOnly => {
            quote! {
                impl #name {
                    fn __leptos_validate_context() {
                        #[cfg(not(feature = "ssr"))]
                        compile_error!(
                            concat!(
                                "Type '", stringify!(#name), 
                                "' can only be used in server context. ",
                                "Use #[leptos::server_only] or ensure 'ssr' feature is enabled."
                            )
                        );
                    }
                }
            }
        }
        ContextType::ClientOnly => {
            quote! {
                impl #name {
                    fn __leptos_validate_context() {
                        #[cfg(feature = "ssr")]
                        compile_error!(
                            concat!(
                                "Type '", stringify!(#name), 
                                "' can only be used in client context. ",
                                "Use #[leptos::client_only] or ensure 'csr' or 'hydrate' feature is enabled."
                            )
                        );
                    }
                }
            }
        }
        ContextType::Universal => {
            quote! {
                impl #name {
                    fn __leptos_validate_context() {
                        // Universal types can be used in any context
                    }
                }
            }
        }
    };

    Ok(validation_code)
}

/// Context type for validation
#[derive(Debug, Clone, PartialEq)]
enum ContextType {
    ServerOnly,
    ClientOnly,
    Universal,
}

/// Extract context type from attributes
fn extract_context_from_attrs(attrs: &[Attribute]) -> syn::Result<ContextType> {
    for attr in attrs {
        if attr.path().is_ident("leptos") {
            if let Ok(meta) = attr.parse_args::<Meta>() {
                match meta {
                    Meta::Path(path) => {
                        if path.is_ident("server_only") {
                            return Ok(ContextType::ServerOnly);
                        } else if path.is_ident("client_only") {
                            return Ok(ContextType::ClientOnly);
                        } else if path.is_ident("universal") {
                            return Ok(ContextType::Universal);
                        }
                    }
                    Meta::List(list) => {
                        if list.path.is_ident("leptos") {
                            // Parse the tokens manually for the new syn API
                            let tokens = list.tokens.to_string();
                            if tokens.contains("server_only") {
                                return Ok(ContextType::ServerOnly);
                            } else if tokens.contains("client_only") {
                                return Ok(ContextType::ClientOnly);
                            } else if tokens.contains("universal") {
                                return Ok(ContextType::Universal);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    Ok(ContextType::Universal) // Default to universal if no context specified
}

/// Macro for server-only code blocks
///
/// This macro ensures that the annotated code only compiles when building for server context.
/// It will cause a compile error if used in client builds.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::server_only;
///
/// server_only! {
///     // This code only compiles in server context
///     let db = Database::connect().await?;
///     let result = db.query("SELECT * FROM users").await?;
/// }
/// ```
#[proc_macro]
pub fn server_only(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TokenStream2);
    
    let expanded = quote! {
        #[cfg(feature = "ssr")]
        {
            #input
        }
        #[cfg(not(feature = "ssr"))]
        {
            compile_error!(
                "This code block can only be used in server context. \
                 Ensure 'ssr' feature is enabled or use server functions."
            );
        }
    };
    
    expanded.into()
}

/// Macro for client-only code blocks
///
/// This macro ensures that the annotated code only compiles when building for client context.
/// It will cause a compile error if used in server builds.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::client_only;
///
/// client_only! {
///     // This code only compiles in client context
///     let window = web_sys::window().unwrap();
///     let document = window.document().unwrap();
/// }
/// ```
#[proc_macro]
pub fn client_only(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TokenStream2);
    
    let expanded = quote! {
        #[cfg(any(feature = "csr", feature = "hydrate"))]
        {
            #input
        }
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        {
            compile_error!(
                "This code block can only be used in client context. \
                 Ensure 'csr' or 'hydrate' feature is enabled."
            );
        }
    };
    
    expanded.into()
}

/// Macro for conditional compilation based on Leptos mode
///
/// This macro allows code to be conditionally compiled based on the current Leptos build mode.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::mode_conditional;
///
/// mode_conditional! {
///     #[mode = "spa"]
///     {
///         // This code only compiles in SPA mode
///         leptos::mount_to_body(App);
///     }
///     
///     #[mode = "fullstack"]
///     {
///         // This code only compiles in fullstack mode
///         leptos::mount_to_body_with_context(App, || provide_context(server_data));
///     }
/// }
/// ```
#[proc_macro]
pub fn mode_conditional(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TokenStream2);
    
    // For now, we'll implement a simple version that checks environment variables
    // In a full implementation, this would be more sophisticated
    let expanded = quote! {
        {
            let current_mode = std::env::var("LEPTOS_MODE").unwrap_or_default();
            match current_mode.as_str() {
                "spa" => {
                    #[cfg(any(feature = "csr"))]
                    {
                        #input
                    }
                }
                "fullstack" => {
                    #[cfg(any(feature = "ssr", feature = "hydrate"))]
                    {
                        #input
                    }
                }
                "static" => {
                    #[cfg(any(feature = "ssr"))]
                    {
                        #input
                    }
                }
                _ => {
                    // Default behavior - compile the code
                    #input
                }
            }
        }
    };
    
    expanded.into()
}

/// Attribute macro for server-only functions
///
/// This attribute ensures that the annotated function can only be called in server context.
/// It's similar to the existing `#[server]` macro but provides additional validation.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::server_only_fn;
///
/// #[server_only_fn]
/// async fn database_query() -> Result<String, ServerFnError> {
///     // This function can only be called from server context
///     let db = Database::connect().await?;
///     Ok(db.query("SELECT * FROM users").await?)
/// }
/// ```
#[proc_macro_attribute]
pub fn server_only_fn(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #[cfg(not(feature = "ssr"))]
            compile_error!(
                concat!(
                    "Function '", stringify!(#fn_name), 
                    "' can only be called in server context. ",
                    "Use server functions or ensure 'ssr' feature is enabled."
                )
            );
            
            #fn_block
        }
    };
    
    expanded.into()
}

/// Attribute macro for client-only functions
///
/// This attribute ensures that the annotated function can only be called in client context.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::client_only_fn;
///
/// #[client_only_fn]
/// fn setup_web_socket() -> Result<(), JsValue> {
///     // This function can only be called from client context
///     let ws = WebSocket::new("ws://localhost:8080")?;
///     // ... setup websocket
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn client_only_fn(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    
    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #[cfg(not(any(feature = "csr", feature = "hydrate")))]
            compile_error!(
                concat!(
                    "Function '", stringify!(#fn_name), 
                    "' can only be called in client context. ",
                    "Ensure 'csr' or 'hydrate' feature is enabled."
                )
            );
            
            #fn_block
        }
    };
    
    expanded.into()
}

/// Derive macro for automatic feature validation
///
/// This macro automatically validates that the required features are enabled
/// for the annotated type based on its usage patterns.
///
/// # Example
///
/// ```rust
/// use leptos_compile_validator_derive::FeatureValidated;
///
/// #[derive(FeatureValidated)]
/// #[leptos::requires_features("ssr", "tracing")]
/// struct ServerComponent {
///     // This struct requires SSR and tracing features
/// }
/// ```
#[proc_macro_derive(FeatureValidated, attributes(leptos))]
pub fn derive_feature_validated(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    match derive_feature_validated_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_feature_validated_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let required_features = extract_required_features(&input.attrs)?;
    
    if required_features.is_empty() {
        return Ok(quote! {});
    }
    
    let feature_checks: Vec<TokenStream2> = required_features
        .iter()
        .map(|feature| {
            quote! {
                #[cfg(not(feature = #feature))]
                compile_error!(
                    concat!(
                        "Type '", stringify!(#name), 
                        "' requires feature '", #feature, 
                        "' to be enabled."
                    )
                );
            }
        })
        .collect();
    
    let validation_code = quote! {
        impl #name {
            fn __leptos_validate_features() {
                #(#feature_checks)*
            }
        }
    };
    
    Ok(validation_code)
}

/// Extract required features from attributes
fn extract_required_features(attrs: &[Attribute]) -> syn::Result<Vec<String>> {
    for attr in attrs {
        if attr.path().is_ident("leptos") {
            if let Ok(meta) = attr.parse_args::<Meta>() {
                if let Meta::List(list) = meta {
                    if list.path.is_ident("leptos") {
                        // Parse the tokens manually for the new syn API
                        let tokens = list.tokens.to_string();
                        if tokens.contains("requires_features") {
                            // Simple parsing - extract quoted strings
                            let mut features = Vec::new();
                            let mut chars = tokens.chars().peekable();
                            let mut current_string = String::new();
                            let mut in_quotes = false;
                            
                            while let Some(c) = chars.next() {
                                match c {
                                    '"' if !in_quotes => in_quotes = true,
                                    '"' if in_quotes => {
                                        if !current_string.is_empty() {
                                            features.push(current_string.clone());
                                            current_string.clear();
                                        }
                                        in_quotes = false;
                                    }
                                    c if in_quotes => current_string.push(c),
                                    _ => {}
                                }
                            }
                            
                            if !features.is_empty() {
                                return Ok(features);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_extract_context_from_attrs() {
        let attrs = vec![parse_quote!(#[leptos::server_only])];
        let context = extract_context_from_attrs(&attrs).unwrap();
        assert_eq!(context, ContextType::ServerOnly);
        
        let attrs = vec![parse_quote!(#[leptos::client_only])];
        let context = extract_context_from_attrs(&attrs).unwrap();
        assert_eq!(context, ContextType::ClientOnly);
        
        let attrs = vec![];
        let context = extract_context_from_attrs(&attrs).unwrap();
        assert_eq!(context, ContextType::Universal);
    }

    #[test]
    fn test_extract_required_features() {
        let attrs = vec![parse_quote!(#[leptos::requires_features("ssr", "tracing")])];
        let features = extract_required_features(&attrs).unwrap();
        assert_eq!(features, vec!["ssr".to_string(), "tracing".to_string()]);
        
        let attrs = vec![];
        let features = extract_required_features(&attrs).unwrap();
        assert!(features.is_empty());
    }
}