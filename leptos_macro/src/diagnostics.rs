//! Custom diagnostic system for Leptos framework error messages
//! 
//! This module provides framework-aware error detection and custom error messages
//! that provide clear, actionable guidance for common Leptos usage mistakes.

use proc_macro2::{Span, TokenStream};
use proc_macro_error2::{abort, emit_warning};
use quote::quote;
use syn::{spanned::Spanned, Expr, ExprPath, ExprField, ExprMethodCall, ExprBlock, Stmt};

/// Custom diagnostic system for Leptos framework errors
pub struct LeptosDiagnostics {
    /// Whether to emit warnings instead of errors for some issues
    pub warn_mode: bool,
}

impl LeptosDiagnostics {
    /// Create a new diagnostics instance
    pub fn new() -> Self {
        Self { warn_mode: false }
    }

    /// Create a diagnostics instance in warning mode
    pub fn warn_mode() -> Self {
        Self { warn_mode: true }
    }

    /// Analyze an expression for common Leptos usage patterns and emit helpful errors
    pub fn analyze_expression(&self, expr: &Expr, span: Span) -> Option<TokenStream> {
        match expr {
            // Detect signal usage without .get()
            Expr::Path(path) => self.check_signal_usage(path, span),
            
            // Detect signal field access without .get()
            Expr::Field(field) => self.check_signal_field_access(field, span),
            
            // Detect method calls that might be signal-related
            Expr::MethodCall(method_call) => self.check_signal_method_call(method_call, span),
            
            // Detect block expressions that might contain signal issues
            Expr::Block(block) => self.check_block_expression(block, span),
            
            _ => None,
        }
    }

    /// Check if a path expression is a signal that should use .get()
    fn check_signal_usage(&self, path: &ExprPath, span: Span) -> Option<TokenStream> {
        let path_str = path_to_string(path);
        
        // Common signal variable names
        if is_likely_signal_name(&path_str) {
            self.emit_signal_usage_error(path_str, span);
            return Some(quote! { 
                compile_error!("Signal used directly in view - use .get() to access value")
            });
        }
        
        None
    }

    /// Check if a field access is on a signal
    fn check_signal_field_access(&self, field: &ExprField, span: Span) -> Option<TokenStream> {
        if let Expr::Path(path) = &*field.base {
            let base_path = path_to_string(path);
            if is_likely_signal_name(&base_path) {
                self.emit_signal_field_access_error(base_path, &field.member, span);
                return Some(quote! { 
                    compile_error!("Signal field access without .get() - use .get().field_name")
                });
            }
        }
        None
    }

    /// Check method calls for signal-related issues
    fn check_signal_method_call(&self, method_call: &ExprMethodCall, span: Span) -> Option<TokenStream> {
        let method_name = method_call.method.to_string();
        
        // Check for signal methods that might be misused
        match method_name.as_str() {
            "set" | "update" => {
                if let Expr::Path(path) = &*method_call.receiver {
                    let path_str = path_to_string(path);
                    if is_likely_signal_name(&path_str) {
                        self.emit_signal_method_usage_warning(path_str, &method_name, span);
                    }
                }
            }
            _ => {}
        }
        
        None
    }

    /// Check block expressions for signal issues
    fn check_block_expression(&self, block: &ExprBlock, _span: Span) -> Option<TokenStream> {
        for stmt in &block.block.stmts {
            if let Stmt::Expr(expr, _) = stmt {
                if let Some(tokens) = self.analyze_expression(expr, expr.span()) {
                    return Some(tokens);
                }
            }
        }
        None
    }

    /// Emit a helpful error for signal usage without .get()
    fn emit_signal_usage_error(&self, signal_name: String, span: Span) {
        let error_msg = format!(
            "Signal '{}' used directly in view without calling .get()",
            signal_name
        );
        
        let help_msg = format!(
            "try `{}.get()` or `move || {}.get()`",
            signal_name, signal_name
        );
        
        let note_msg = "Signals need to be read with .get() to access their values in views";
        
        let help_dynamic = format!("For dynamic content, use: `{{move || {}.get()}}`", signal_name);
        let help_static = format!("For one-time reads, use: `{{{}.get()}}`", signal_name);
        
        if self.warn_mode {
            emit_warning!(
                span,
                "{}", error_msg;
                help = "{}", help_msg;
                note = "{}", note_msg;
                help = "{}", help_dynamic;
                help = "{}", help_static;
                note = "docs: https://leptos.dev/docs/reactivity/signals"
            );
        } else {
            abort!(
                span,
                "{}", error_msg;
                help = "{}", help_msg;
                note = "{}", note_msg;
                help = "{}", help_dynamic;
                help = "{}", help_static;
                note = "docs: https://leptos.dev/docs/reactivity/signals"
            );
        }
    }

    /// Emit a helpful error for signal field access without .get()
    fn emit_signal_field_access_error(&self, signal_name: String, field: &syn::Member, span: Span) {
        let field_str = match field {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        
        let error_msg = format!(
            "Signal '{}' field access without calling .get()",
            signal_name
        );
        
        let help_msg = format!(
            "try `{}.get().{}`",
            signal_name, field_str
        );
        
        let note_msg = "Access signal fields after calling .get() to get the actual value";
        
        if self.warn_mode {
            emit_warning!(
                span,
                "{}", error_msg;
                help = "{}", help_msg;
                note = "{}", note_msg;
                note = "docs: https://leptos.dev/docs/reactivity/signals"
            );
        } else {
            abort!(
                span,
                "{}", error_msg;
                help = "{}", help_msg;
                note = "{}", note_msg;
                note = "docs: https://leptos.dev/docs/reactivity/signals"
            );
        }
    }

    /// Emit a warning for signal method usage in views
    fn emit_signal_method_usage_warning(&self, signal_name: String, method: &str, span: Span) {
        let warning_msg = format!(
            "Signal '{}' method '{}' called in view - consider using effects instead",
            signal_name, method
        );
        
        let help_msg = format!(
            "For reactive updates, use create_effect: `create_effect(move |_| {}.{}(new_value))`",
            signal_name, method
        );
        
        let note_msg = "Signal mutations in views can cause performance issues";
        
        emit_warning!(
            span,
            "{}", warning_msg;
            help = "{}", help_msg;
            note = "{}", note_msg;
            note = "docs: https://leptos.dev/docs/common-patterns#effects"
        );
    }
}

/// Convert a path expression to a string representation
fn path_to_string(path: &ExprPath) -> String {
    path.path
        .segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Check if a variable name is likely a signal
fn is_likely_signal_name(name: &str) -> bool {
    // Common signal naming patterns
    let signal_patterns = [
        "count", "value", "data", "state", "loading", "error", "user", "items",
        "selected", "active", "visible", "enabled", "disabled", "open", "closed",
        "current", "previous", "next", "first", "last", "total", "sum", "result",
        "response", "request", "form", "input", "output", "config", "settings",
    ];
    
    // Check for exact matches
    if signal_patterns.contains(&name) {
        return true;
    }
    
    // Check for common signal suffixes
    if name.ends_with("_signal") || name.ends_with("_state") || name.ends_with("_data") {
        return true;
    }
    
    // Check for common signal prefixes
    if name.starts_with("is_") || name.starts_with("has_") || name.starts_with("should_") {
        return true;
    }
    
    false
}

/// Server function diagnostic system
pub struct ServerFunctionDiagnostics;

impl ServerFunctionDiagnostics {
    /// Check for server function usage issues
    pub fn check_server_function_usage(&self, expr: &Expr, span: Span) -> Option<TokenStream> {
        match expr {
            Expr::Path(path) => {
                let path_str = path_to_string(path);
                if is_likely_server_function(&path_str) {
                    self.emit_server_function_error(path_str, span);
                    return Some(quote! { 
                        compile_error!("Server function called in client context")
                    });
                }
            }
            Expr::MethodCall(method_call) => {
                if let Expr::Path(path) = &*method_call.receiver {
                    let path_str = path_to_string(path);
                    if is_likely_server_function(&path_str) {
                        self.emit_server_function_error(path_str, span);
                        return Some(quote! { 
                            compile_error!("Server function called in client context")
                        });
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Emit helpful error for server function usage
    fn emit_server_function_error(&self, func_name: String, span: Span) {
        let error_msg = format!(
            "Server function '{}' called in client context",
            func_name
        );
        
        let help_msg = format!(
            "To load server data on the client, use a Resource: `let data = Resource::new(|| (), |_| {}());`",
            func_name
        );
        
        let help_access = "Then access with: `data.get()`";
        let note_msg = "Server functions are not directly callable from client code";
        
        abort!(
            span,
            "{}", error_msg;
            help = "{}", help_msg;
            help = "{}", help_access;
            note = "{}", note_msg;
            note = "docs: https://leptos.dev/docs/server-functions"
        );
    }
}

/// Check if a function name is likely a server function
fn is_likely_server_function(name: &str) -> bool {
    let server_patterns = [
        "get_data", "fetch_data", "load_data", "save_data", "update_data", "delete_data",
        "get_user", "fetch_user", "load_user", "save_user", "update_user", "delete_user",
        "get_posts", "fetch_posts", "load_posts", "save_posts", "update_posts", "delete_posts",
        "get_config", "fetch_config", "load_config", "save_config", "update_config",
        "get_settings", "fetch_settings", "load_settings", "save_settings", "update_settings",
        "login", "logout", "register", "authenticate", "authorize",
        "upload", "download", "export", "import", "sync",
    ];
    
    server_patterns.contains(&name)
}

/// Configuration validation diagnostics
pub struct ConfigurationDiagnostics;

impl ConfigurationDiagnostics {
    /// Check for conflicting feature flags
    pub fn check_feature_conflicts(&self, features: &[String], span: Span) {
        let has_csr = features.contains(&"csr".to_string());
        let has_ssr = features.contains(&"ssr".to_string());
        let has_static = features.contains(&"static".to_string());
        
        if has_csr && has_ssr && !has_static {
            self.emit_feature_conflict_error(span);
        }
    }

    /// Emit error for conflicting features
    fn emit_feature_conflict_error(&self, span: Span) {
        let error_msg = "Conflicting Leptos features enabled";
        let help_csr = "For SPAs: default = [\"csr\"]";
        let help_ssr = "For SSR: default = [\"ssr\"]";
        let help_ssg = "For SSG: default = [\"ssr\", \"static\"]";
        let note_msg = "Use separate build configurations for different deployment targets";
        
        abort!(
            span,
            "{}", error_msg;
            help = "{}", help_csr;
            help = "{}", help_ssr;
            help = "{}", help_ssg;
            note = "{}", note_msg;
            note = "docs: https://leptos.dev/docs/deployment"
        );
    }
}
