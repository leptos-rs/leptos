//! Per-prop type-check code generation for the companion module
//! pattern.
//!
//! Each prop gets a wrapper struct + helper method. Some props also
//! get a check trait with `#[diagnostic::on_unimplemented]` for
//! custom error messages. See [`PropClassification`] for the three
//! strategies and [`generate_prop_checks`] for the entry point.

use super::{strip_raw_prefix, type_analysis, PropLike};
use proc_macro2::{Group, Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{GenericParam, Generics, Type};

/// The generated token streams for per-prop companion checks.
#[derive(Default)]
pub(crate) struct PropCheckOutput {
    /// Trait definitions for inside the companion module.
    pub check_traits: Vec<TokenStream>,
    /// Trait implementations for outside the companion module.
    pub trait_impls: Vec<TokenStream>,
    /// Wrapper struct definitions and their impl blocks for inside the
    /// companion module.
    pub wrapper_items: Vec<TokenStream>,
    /// Check/wrap methods for the `Helper` struct's unbounded impl
    /// block. These take `&self` and use module-local paths (no module
    /// prefix) since they live inside the companion module.
    pub unbounded_helper_methods: Vec<TokenStream>,
    /// Check/wrap methods that need a bounded impl block on
    /// `Helper` for closure parameter inference. These are
    /// `PropClassification::DependentBounds` methods whose prop type
    /// is a generic type parameter with bounds that reference other
    /// generic params. They all share ONE bounded impl block with ALL
    /// the original where-clause predicates, so cross-parameter
    /// inference works (e.g. `T` gets resolved from `each`'s
    /// `IF: Fn() -> I`, `I: IntoIterator<Item = T>` and is then
    /// available for `key`'s `KF: Fn(&T) -> K`).
    pub bounded_helper_methods: Vec<TokenStream>,
}

/// Generates per-prop check traits and their implementations for
/// the companion module pattern.
///
/// For each prop (see [`PropClassification`]):
/// - **`IndependentBounds`**: marker-only `Check_*` trait (with
///   `on_unimplemented` and supertraits) inside module; bounded marker impl
///   outside. Wrapper struct provides `{error}` propagation via bounded
///   `extract_value()`. Helper method takes the value with `Check_*` trait
///   bound (supertraits imply actual bounds, enabling closure parameter
///   inference).
/// - **`DependentBounds`**: no trait generated. Wrapper struct with blanket
///   `extract_value()`. Helper method takes the value as the struct's type
///   parameter directly (not a fresh generic), preserving closure parameter
///   inference for closures with untyped params.
/// - **`DeferredToBuilder`**: no trait generated. Wrapper struct with blanket
///   `extract_value()`. Helper method takes any type with no bound.
///
///  # Parameters
///
/// - `full_generics`: the full generics from the original function/struct (all
///   bounds)
/// - `module_name`: name of the companion module (e.g. `ComponentName` for
///   components, `__SlotName` for slots)
/// - `display_name`: human-readable name for error messages (e.g.
///   `ComponentName` for components, `SlotName` for slots)
/// - `props`: (name, type) pairs for each prop
/// - `field_types`: all field types (for structural bounds check)
pub(crate) fn generate_prop_checks<'a, P: PropLike>(
    full_generics: &Generics,
    module_name: &Ident,
    display_name: &Ident,
    kind: &str,
    props: &'a [P],
    field_types: &[&Type],
) -> PropCheckOutput {
    if props.is_empty() {
        return PropCheckOutput::default();
    }

    let mut output = PropCheckOutput::default();

    for (prop_name, prop_ty) in props.iter().map(|p| (p.name(), p.ty())) {
        let clean_name = strip_raw_prefix(prop_name);
        let classification =
            PropClassification::classify(prop_ty, field_types, full_generics);

        // Intentionally call_site span: these synthetic idents should
        // not link back to any user source location.
        let wrap_struct_name = format_ident!("Wrap_{}", clean_name);
        let check_and_wrap_name =
            format_ident!("check_and_wrap_{}", clean_name);

        // Wrapper struct — identical for all classifications.
        output.wrapper_items.push(quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            pub struct #wrap_struct_name<__T>(pub __T);
        });

        match classification {
            PropClassification::IndependentBounds {
                bounds,
                has_fn_bound,
            } => {
                let check_trait_name = format_ident!("Check_{}", clean_name);
                let bounds_note = bounds.to_string();
                let message = format!(
                    "`{{Self}}` is not a valid type for prop `{clean_name}` \
                     on {kind} `{display_name}`",
                );
                let hint = if has_fn_bound {
                    " — a closure or function reference"
                } else {
                    ""
                };
                let note = format!("required: `{bounds_note}`{hint}");

                // Despan bounds to call_site so compiler notes don't
                // produce multi-line spans back to the original
                // function signature. The bound info is already in
                // our custom on_unimplemented note text.
                let bounds_despanned = despan(bounds.clone());

                // Marker-only trait with on_unimplemented and supertraits.
                output.check_traits.push(quote! {
                    #[doc(hidden)]
                    #[diagnostic::on_unimplemented(
                        message = #message,
                        note = #note
                    )]
                    #[allow(non_camel_case_types)]
                    pub trait #check_trait_name: #bounds {}
                });

                // Outside module: bounded marker impl.
                output.trait_impls.push(quote! {
                    #[doc(hidden)]
                    impl<__T: #bounds_despanned>
                        #module_name::#check_trait_name for __T
                    {}
                });

                // Bounded extract_value() — when bounds fail →
                // E0599 → {error}.
                output.wrapper_items.push(quote! {
                    #[doc(hidden)]
                    impl<__T: #bounds_despanned> #wrap_struct_name<__T> {
                        pub fn extract_value(self) -> __T { self.0 }
                    }
                });

                // Unbounded check+wrap helper method. The trait bound
                // is satisfied by supertraits on Check_*.
                output.unbounded_helper_methods.push(quote! {
                    #[doc(hidden)]
                    pub fn #check_and_wrap_name<__T: #check_trait_name>(
                        &self, val: __T,
                    ) -> #wrap_struct_name<__T> {
                        #wrap_struct_name(val)
                    }
                });
            }
            PropClassification::DependentBounds { type_param } => {
                output
                    .wrapper_items
                    .push(blanket_extract_value(&wrap_struct_name));

                // No trait needed. The helper method takes the value
                // as the struct's type parameter directly, preserving
                // closure parameter inference.
                output.bounded_helper_methods.push(quote! {
                    #[doc(hidden)]
                    pub fn #check_and_wrap_name(
                        &self, val: #type_param,
                    ) -> #wrap_struct_name<#type_param> {
                        #wrap_struct_name(val)
                    }
                });
            }
            PropClassification::DeferredToBuilder => {
                output
                    .wrapper_items
                    .push(blanket_extract_value(&wrap_struct_name));

                // No trait needed — accept any type. Actual type
                // checking is deferred to TypedBuilder's setter.
                output.unbounded_helper_methods.push(quote! {
                    #[doc(hidden)]
                    pub fn #check_and_wrap_name<__T>(
                        &self, val: __T,
                    ) -> #wrap_struct_name<__T> {
                        #wrap_struct_name(val)
                    }
                });
            }
        }
    }

    output
}

/// Generates a blanket `extract_value()` impl that passes all types through.
///
/// Used by both `DependentBounds` and `DeferredToBuilder` classifications.
fn blanket_extract_value(wrap_struct_name: &Ident) -> TokenStream {
    quote! {
        #[doc(hidden)]
        impl<__T> #wrap_struct_name<__T> {
            pub fn extract_value(self) -> __T { self.0 }
        }
    }
}

/// Classifies a component/slot prop to determine how per-prop type-check
/// code is generated in the companion module.
///
/// The `view!` macro generates `check_and_wrap_foo(val).into_inner()` calls
/// for ALL props uniformly — it doesn't know prop types. This classification
/// determines what code backs those calls: a custom-error check trait,
/// a dependent-bounds passthrough, or a no-op.
///
/// The three variants correspond to three fundamentally different compiler
/// behaviors when checking prop types, each requiring a different code
/// generation strategy to produce clear, localized error messages.
enum PropClassification {
    /// The prop's type is a generic type parameter used directly as the
    /// prop type (e.g. `F`, not `Vec<F>`) whose bounds reference no
    /// other generics of the component.
    ///
    /// Example: `F: Fn() -> bool` or `T: Clone + Display`.
    ///
    /// Because the bounds are self-contained, we can express them as
    /// supertraits on a dedicated check trait with a custom
    /// `#[diagnostic::on_unimplemented]` message. This is the only
    /// classification that enables both custom error messages and
    /// `{error}` propagation to suppress downstream errors.
    IndependentBounds {
        /// The token stream for the bounds (e.g. `Fn() -> bool`).
        /// Used as supertraits on the check trait and in the bounded
        /// impl.
        bounds: TokenStream,

        /// Whether any of the bounds are `Fn`/`FnMut`/`FnOnce`
        /// traits. Used to append a hint ("a closure or function
        /// reference") to the error note.
        has_fn_bound: bool,
    },

    /// The prop's type is a generic type parameter used directly as the
    /// prop type (e.g. `KF`, not `Vec<KF>`) whose bounds reference
    /// other generic parameters of the component (e.g. `KF: Fn(&T) -> K`
    /// where `T` comes from another param).
    ///
    /// Custom error messages are not possible here because the bounds
    /// cannot be expressed as self-contained supertraits. Instead, the
    /// helper method's signature uses the component's own type
    /// parameter (e.g. `val: KF`) rather than introducing a new
    /// method-level generic (e.g. `val: __T`). This preserves the
    /// compiler's ability to infer closure parameter types through
    /// the full predicate chain.
    DependentBounds {
        /// The original type parameter ident (e.g. `KF`). The helper
        /// method's signature uses this directly instead of a fresh
        /// generic, preserving inference.
        type_param: Ident,
    },

    /// The prop's type is either concrete (e.g. `String`), a
    /// composite generic type (e.g. `Vec<T>`), or a generic type
    /// parameter with no bounds.
    ///
    /// No custom type checking is performed at the helper level —
    /// the helper method simply accepts any type. Actual type checking
    /// is deferred to TypedBuilder's setter method.
    DeferredToBuilder,
}

impl PropClassification {
    /// Classifies a prop for check generation.
    fn classify(
        prop_type: &Type,
        field_types: &[&Type],
        full_generics: &Generics,
    ) -> PropClassification {
        // Find a generic type parameter that is used directly as this
        // prop's type (not wrapped in another type like `Vec<T>`), and
        // whose bounds have not been kept on the struct (i.e. it was
        // stripped because it doesn't appear wrapped in any field).
        let stripped_param = full_generics.params.iter().find_map(|p| {
            if let GenericParam::Type(tp) = p {
                let ident = &tp.ident;
                if type_analysis::is_exact_type_param(prop_type, ident)
                    && !type_analysis::param_appears_wrapped_in_fields(
                        ident,
                        field_types,
                    )
                {
                    return Some(ident);
                }
            }
            None
        });

        let classification = if let Some(param_ident) = stripped_param {
            let preds = type_analysis::collect_predicates_for_param(
                full_generics,
                param_ident,
            );
            if !preds.is_empty()
                && !type_analysis::bounds_reference_other_params(
                    &preds,
                    param_ident,
                    full_generics,
                )
            {
                let bounds = type_analysis::merge_predicate_bounds(&preds);
                let has_fn_bound =
                    type_analysis::predicates_contain_fn_bound(&preds);
                PropClassification::IndependentBounds {
                    bounds,
                    has_fn_bound,
                }
            } else {
                PropClassification::DependentBounds {
                    type_param: (*param_ident).clone(),
                }
            }
        } else {
            PropClassification::DeferredToBuilder
        };

        classification
    }
}

/// Resets all spans in a token stream to `Span::call_site()`.
fn despan(ts: TokenStream) -> TokenStream {
    ts.into_iter()
        .map(|mut tt| {
            if let TokenTree::Group(g) = tt {
                let mut new = Group::new(g.delimiter(), despan(g.stream()));
                new.set_span(Span::call_site());
                tt = TokenTree::Group(new);
            } else {
                tt.set_span(Span::call_site());
            }
            tt
        })
        .collect()
}
