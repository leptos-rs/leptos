//! Shared utility functions for the `#[component]` and `#[slot]`
//! macros.
//!
//! These helpers are used by both `component.rs` and `slot.rs` to
//! generate per-prop type checks, required-prop checks, and phantom
//! fields — all part of the localized error reporting machinery.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{visit::Visit, GenericParam, Type, TypePath, WherePredicate};

// -------------------------------------------------------------------
// Small helpers
// -------------------------------------------------------------------

/// Strips the raw identifier prefix (`r#`) from a prop name.
pub(crate) fn clean_prop_name(ident: &Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").unwrap_or(&s).to_owned()
}

/// Extracts generic arguments (idents / lifetimes / const idents)
/// for type position (no bounds).
pub(crate) fn generic_arg_tokens(generics: &syn::Generics) -> Vec<TokenStream> {
    generics
        .params
        .iter()
        .map(|p| match p {
            GenericParam::Type(tp) => {
                let i = &tp.ident;
                quote! { #i }
            }
            GenericParam::Lifetime(lp) => {
                let lt = &lp.lifetime;
                quote! { #lt }
            }
            GenericParam::Const(cp) => {
                let i = &cp.ident;
                quote! { #i }
            }
        })
        .collect()
}

// -------------------------------------------------------------------
// Type analysis helpers
// -------------------------------------------------------------------

/// Walks a syntax tree looking for path segments matching any of the
/// given target identifiers.
///
/// Only `visit_path_segment` is overridden because generic type
/// params always appear as path segments in type ASTs (e.g. `F`,
/// `Vec<F>`, `Option<F>`). This is sufficient for detecting whether
/// a generic param is referenced.
struct IdentFinder<'a> {
    targets: &'a [&'a Ident],
    found: bool,
}

impl<'ast> Visit<'ast> for IdentFinder<'_> {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if self.targets.iter().any(|t| segment.ident == **t) {
            self.found = true;
        }
        syn::visit::visit_path_segment(self, segment);
    }
}

/// Checks whether a type AST contains a reference to the given
/// identifier (used to detect which generic params appear in field
/// types).
pub(crate) fn type_contains_ident(ty: &Type, ident: &Ident) -> bool {
    let mut finder = IdentFinder {
        targets: &[ident],
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

/// Returns true if the type is exactly the given generic param name
/// with no wrapping.
pub(crate) fn is_bare_generic_param(ty: &Type, ident: &Ident) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.is_ident(ident)
    } else {
        false
    }
}

/// A generic param needs structural bounds if it appears in a field
/// type but NOT as a bare type param — meaning it is wrapped inside
/// another generic type (e.g. `ServerAction<ServFn>`).
pub(crate) fn param_needs_structural_bounds(
    param_ident: &Ident,
    field_types: &[&Type],
) -> bool {
    for ty in field_types {
        if type_contains_ident(ty, param_ident)
            && !is_bare_generic_param(ty, param_ident)
        {
            return true;
        }
    }
    false
}

/// Creates a copy of the generics keeping only the bounds that are
/// structurally required by field types.
///
/// A generic param needs structural bounds when it appears inside
/// another generic type in a field (e.g. `ServerAction<ServFn>` needs
/// `ServFn: ServerFn`). Bare type params (e.g. `fun: F`) do not
/// need their bounds on the struct definition.
pub(crate) fn strip_non_structural_bounds(
    generics: &syn::Generics,
    field_types: &[&Type],
) -> syn::Generics {
    let mut g = generics.clone();

    // Strip inline bounds for params that don't need structural
    // bounds.
    for param in &mut g.params {
        if let GenericParam::Type(tp) = param {
            if !param_needs_structural_bounds(&tp.ident, field_types) {
                tp.bounds.clear();
                tp.colon_token = None;
            }
        }
    }

    // Filter where-clause predicates.
    if let Some(where_clause) = &mut g.where_clause {
        let kept: syn::punctuated::Punctuated<_, _> = where_clause
            .predicates
            .iter()
            .filter(|pred| match pred {
                WherePredicate::Type(pt) => {
                    if let Type::Path(TypePath {
                        path, qself: None, ..
                    }) = &pt.bounded_ty
                    {
                        if let Some(ident) = path.get_ident() {
                            return param_needs_structural_bounds(
                                ident,
                                field_types,
                            );
                        }
                    }
                    // Complex predicates (associated types, etc.)
                    // are kept to be safe.
                    true
                }
                // Keep lifetime bounds and any other predicates.
                _ => true,
            })
            .cloned()
            .collect();

        if kept.is_empty() {
            g.where_clause = None;
        } else {
            where_clause.predicates = kept;
        }
    }

    g
}

/// Collects where-clause predicates (both inline bounds and
/// where-clause entries) for a specific generic type param.
pub(crate) fn collect_predicates_for_param(
    generics: &syn::Generics,
    param_ident: &Ident,
) -> Vec<WherePredicate> {
    let mut preds = vec![];

    // Inline bounds (e.g., `F: Clone` in `<F: Clone>`)
    for param in &generics.params {
        if let GenericParam::Type(tp) = param {
            if tp.ident == *param_ident && !tp.bounds.is_empty() {
                let bounds = &tp.bounds;
                preds.push(syn::parse_quote! {
                    #param_ident: #bounds
                });
            }
        }
    }

    // Where-clause predicates
    if let Some(wc) = &generics.where_clause {
        for pred in &wc.predicates {
            if let WherePredicate::Type(pt) = pred {
                if let Type::Path(TypePath { path, .. }) = &pt.bounded_ty {
                    if path.get_ident() == Some(param_ident) {
                        preds.push(pred.clone());
                    }
                }
            }
        }
    }

    preds
}

/// Collects generic type params that are NOT used in any field type.
/// These need `PhantomData` so the struct compiles without their
/// where-clause bounds.
pub(crate) fn collect_phantom_type_params<'a>(
    generics: &'a syn::Generics,
    field_types: &[&Type],
) -> Vec<&'a Ident> {
    generics
        .params
        .iter()
        .filter_map(|param| {
            if let GenericParam::Type(tp) = param {
                let ident = &tp.ident;
                let used_in_field =
                    field_types.iter().any(|ty| type_contains_ident(ty, ident));
                if !used_in_field {
                    Some(ident)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Generates the `PhantomData` field for a props struct if any
/// generic type params are unused in field types.
pub(crate) fn generate_phantom_field(
    phantom_type_params: &[&Ident],
    add_serde_skip: bool,
) -> TokenStream {
    if phantom_type_params.is_empty() {
        quote! {}
    } else {
        let serde_skip = if add_serde_skip {
            quote! { #[serde(skip)] }
        } else {
            quote! {}
        };
        quote! {
            #[builder(setter(skip), default)]
            #[doc(hidden)]
            #serde_skip
            _phantom: ::core::marker::PhantomData<(#(#phantom_type_params),*)>,
        }
    }
}

/// Checks if any of the bounds in the given predicates reference
/// generic params other than `self_ident`.
pub(crate) fn bounds_reference_other_params(
    predicates: &[WherePredicate],
    self_ident: &Ident,
    all_generic_idents: &[&Ident],
) -> bool {
    let other_idents: Vec<&Ident> = all_generic_idents
        .iter()
        .filter(|i| **i != self_ident)
        .copied()
        .collect();

    if other_idents.is_empty() {
        return false;
    }

    let mut finder = IdentFinder {
        targets: &other_idents,
        found: false,
    };

    for pred in predicates {
        if let WherePredicate::Type(pt) = pred {
            for bound in &pt.bounds {
                finder.visit_type_param_bound(bound);
            }
        }
    }

    finder.found
}

/// Extracts all type-param bounds from a list of where predicates
/// and combines them with `+`.
pub(crate) fn predicates_to_bounds(
    predicates: &[WherePredicate],
) -> TokenStream {
    let all_bounds: Vec<&syn::TypeParamBound> = predicates
        .iter()
        .filter_map(|pred| {
            if let WherePredicate::Type(pt) = pred {
                Some(pt.bounds.iter())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    if all_bounds.is_empty() {
        quote! {}
    } else {
        quote! { #(#all_bounds)+* }
    }
}

// -------------------------------------------------------------------
// Prop classification
// -------------------------------------------------------------------

/// Shared preamble data for prop classification.
pub(crate) struct PropCheckPreamble<'a> {
    pub all_generic_idents: Vec<&'a Ident>,
    pub stripped_params: Vec<&'a Ident>,
}

/// Computes the shared preamble for prop check generation:
/// identifies all generic type idents and which ones have been
/// stripped of non-structural bounds.
pub(crate) fn prop_check_preamble<'a>(
    full_generics: &'a syn::Generics,
    field_types: &[&Type],
) -> PropCheckPreamble<'a> {
    let all_generic_idents: Vec<&Ident> = full_generics
        .params
        .iter()
        .filter_map(|p| {
            if let GenericParam::Type(tp) = p {
                Some(&tp.ident)
            } else {
                None
            }
        })
        .collect();

    let stripped_params: Vec<&Ident> = all_generic_idents
        .iter()
        .copied()
        .filter(|ident| !param_needs_structural_bounds(ident, field_types))
        .collect();

    PropCheckPreamble {
        all_generic_idents,
        stripped_params,
    }
}

/// Classification result for a single prop.
pub(crate) enum PropClassification {
    /// Bounded single-param generic: custom `on_unimplemented` +
    /// `{error}` propagation.
    BoundedSingleParam {
        bounds: TokenStream,
        message: String,
        note: String,
    },
    /// Everything else: identity pass-through.
    PassThrough,
}

/// Classifies a prop for check generation.
///
/// Returns `(clean_name, classification)`.
pub(crate) fn classify_prop(
    prop_name: &Ident,
    prop_ty: &Type,
    display_name: &Ident,
    preamble: &PropCheckPreamble,
    full_generics: &syn::Generics,
) -> (String, PropClassification) {
    let clean_name = clean_prop_name(prop_name);

    let bounded_single_param = preamble
        .stripped_params
        .iter()
        .find(|ident| is_bare_generic_param(prop_ty, ident))
        .and_then(|param_ident| {
            let preds =
                collect_predicates_for_param(full_generics, param_ident);
            if !preds.is_empty()
                && !bounds_reference_other_params(
                    &preds,
                    param_ident,
                    &preamble.all_generic_idents,
                )
            {
                Some(preds)
            } else {
                None
            }
        });

    let classification = if let Some(param_predicates) = bounded_single_param {
        let bounds = predicates_to_bounds(&param_predicates);
        let bounds_note = bounds.to_string();
        let message = format!(
            "`{{Self}}` is not a valid type for prop `{clean_name}` on \
             component `{display_name}`",
        );
        let note = format!("required: `{bounds_note}`");

        PropClassification::BoundedSingleParam {
            bounds,
            message,
            note,
        }
    } else {
        PropClassification::PassThrough
    };

    (clean_name, classification)
}

// -------------------------------------------------------------------
// Module-based check generation (shared by components and slots)
// -------------------------------------------------------------------

/// The generated token streams for module-based companion checks.
pub(crate) struct ModuleCheckTokens {
    /// Trait definitions for inside the companion module.
    pub module_check_traits: Vec<TokenStream>,
    /// Trait implementations for outside the companion module.
    pub check_trait_impls: Vec<TokenStream>,
}

/// Generates per-prop check traits and their implementations for
/// the companion module pattern.
///
/// For each prop:
/// - **Bounded single-param generic**: `__Check_*` trait (with
///   `on_unimplemented` and `__check_*(&self)`) + `__Pass_*`
///   method trait inside module; bounded `__Check_*` impl +
///   blanket `__Pass_*` impl (bounded on `__Check_*`) outside.
///   View macro calls UFCS:
///   `<_ as Module::__Check_foo>::__check_foo(&value)` — E0277
///   with `on_unimplemented` (works for all types including
///   closures), then method: `value.__pass_foo()` — E0599
///   produces `{error}` for downstream suppression.
/// - **Everything else**: blanket `__Pass_*` trait inside module
///   with blanket impl outside. All types pass through.
///
/// - `full_generics`: the full generics from the original
///   function/struct (all bounds)
/// - `module_name`: name of the companion module (e.g.
///   `ComponentName` for components, `__SlotName` for slots)
/// - `display_name`: human-readable name for error messages (e.g.
///   `ComponentName` for components, `SlotName` for slots)
/// - `props`: (name, type) pairs for each prop
/// - `field_types`: all field types (for structural bounds check)
pub(crate) fn generate_module_checks(
    full_generics: &syn::Generics,
    module_name: &Ident,
    display_name: &Ident,
    props: &[(&Ident, &Type)],
    field_types: &[&Type],
) -> ModuleCheckTokens {
    if props.is_empty() {
        return ModuleCheckTokens {
            module_check_traits: vec![],
            check_trait_impls: vec![],
        };
    }

    let preamble = prop_check_preamble(full_generics, field_types);

    let mut module_check_traits = vec![];
    let mut check_trait_impls = vec![];

    for (prop_name, prop_ty) in props {
        let (clean_name, classification) = classify_prop(
            prop_name,
            prop_ty,
            display_name,
            &preamble,
            full_generics,
        );

        let check_trait_name = format_ident!("__Check_{}", clean_name);
        let check_method_name = format_ident!("__check_{}", clean_name);
        let pass_trait_name = format_ident!("__Pass_{}", clean_name);
        let pass_method_name = format_ident!("__pass_{}", clean_name);

        match classification {
            PropClassification::BoundedSingleParam {
                bounds,
                message,
                note,
            } => {
                // Inside module: check trait (UFCS, E0277) +
                // pass trait (method, E0599 → {error})
                module_check_traits.push(quote! {
                    #[doc(hidden)]
                    #[diagnostic::on_unimplemented(
                        message = #message,
                        note = #note
                    )]
                    #[allow(non_camel_case_types)]
                    pub trait #check_trait_name {
                        fn #check_method_name(&self);
                    }

                    #[doc(hidden)]
                    #[diagnostic::on_unimplemented(
                        message = #message,
                        note = #note
                    )]
                    #[allow(non_camel_case_types)]
                    pub trait #pass_trait_name {
                        fn #pass_method_name(self) -> Self;
                    }
                });

                // Outside module: bounded check impl +
                // blanket pass impl (bounded on check)
                check_trait_impls.push(quote! {
                    #[doc(hidden)]
                    impl<__T: #bounds>
                        #module_name::#check_trait_name for __T
                    {
                        fn #check_method_name(&self) {}
                    }

                    #[doc(hidden)]
                    impl<__T: #module_name::#check_trait_name>
                        #module_name::#pass_trait_name for __T
                    {
                        fn #pass_method_name(self) -> Self {
                            self
                        }
                    }
                });
            }
            PropClassification::PassThrough => {
                // Blanket check + pass traits — all types pass.
                module_check_traits.push(quote! {
                    #[doc(hidden)]
                    #[allow(non_camel_case_types)]
                    pub trait #check_trait_name {
                        fn #check_method_name(&self);
                    }

                    #[doc(hidden)]
                    #[allow(non_camel_case_types)]
                    pub trait #pass_trait_name {
                        fn #pass_method_name(self) -> Self;
                    }
                });

                check_trait_impls.push(quote! {
                    #[doc(hidden)]
                    impl<__T> #module_name::#check_trait_name
                        for __T
                    {
                        fn #check_method_name(&self) {}
                    }

                    #[doc(hidden)]
                    impl<__T> #module_name::#pass_trait_name
                        for __T
                    {
                        fn #pass_method_name(self) -> Self {
                            self
                        }
                    }
                });
            }
        }
    }

    ModuleCheckTokens {
        module_check_traits,
        check_trait_impls,
    }
}

// -------------------------------------------------------------------
// Module-based required check generation
// -------------------------------------------------------------------

/// The split output of `generate_module_required_check`.
pub(crate) struct ModuleRequiredCheckTokens {
    /// Marker trait definitions (with `on_unimplemented`) at module
    /// scope, outside the companion module.
    pub marker_traits: TokenStream,
    /// Items inside the companion module: `__CheckAllRequired`
    /// trait, `__CheckMissing` trait, `__require_props` fn.
    pub module_items: TokenStream,
    /// `impl __CheckAllRequired for PropsBuilder` outside module.
    pub check_all_required_impl: TokenStream,
    /// `impl __CheckMissing for PropsBuilder` outside module.
    pub check_missing_impl: TokenStream,
}

/// Generates module-internal traits and outer impls for
/// required-prop checking.
///
/// `__require_props` triggers E0277 with custom `on_unimplemented`
/// when required props are missing. `__CheckMissing` produces
/// `{error}` via UFCS for downstream suppression.
///
/// - `module_name`: name of the companion module
/// - `display_name`: human-readable name for error messages
/// - `builder_name`: the builder struct name
/// - `generics`: the full generics
/// - `fields`: `(name, is_required)` for each field
pub(crate) fn generate_module_required_check(
    module_name: &Ident,
    display_name: &Ident,
    builder_name: &Ident,
    generics: &syn::Generics,
    fields: &[(&Ident, bool)],
) -> ModuleRequiredCheckTokens {
    if fields.is_empty() {
        return ModuleRequiredCheckTokens {
            marker_traits: quote! {},
            module_items: quote! {
                #[doc(hidden)]
                pub trait __CheckAllRequired {}

                #[doc(hidden)]
                pub trait __CheckMissing {
                    fn __check_missing(self) -> Self;
                }

                #[doc(hidden)]
                #[allow(non_snake_case)]
                pub fn __require_props<__B: __CheckAllRequired>(
                    _: &__B,
                ) {
                }
            },
            check_all_required_impl: quote! {
                impl<__T>
                    #module_name::__CheckAllRequired for __T
                {
                }
            },
            check_missing_impl: quote! {
                impl<__T>
                    #module_name::__CheckMissing for __T
                {
                    fn __check_missing(self) -> Self { self }
                }
            },
        };
    }

    let (_, _, where_clause) = generics.split_for_impl();

    let generic_params: Vec<&GenericParam> = generics.params.iter().collect();

    let type_args = generic_arg_tokens(generics);

    let mut marker_traits = vec![];
    let mut type_state_params = vec![];
    let mut type_state_idents = vec![];

    for (i, (name, required)) in fields.iter().enumerate() {
        let param = format_ident!("__F{}", i);
        type_state_idents.push(param.clone());

        if *required {
            let clean_name = clean_prop_name(name);
            let trait_name = Ident::new(
                &format!("__required_{display_name}_{clean_name}"),
                Span::call_site(),
            );

            let message = format!(
                "missing required prop `{clean_name}` on component \
                 `{display_name}`"
            );
            let label = format!("missing prop `{clean_name}`");
            let note = "all required props must be provided as attributes on \
                        the component";

            marker_traits.push(quote! {
                #[doc(hidden)]
                #[diagnostic::on_unimplemented(
                    message = #message,
                    label = #label,
                    note = #note
                )]
                #[allow(non_camel_case_types)]
                pub trait #trait_name {}

                impl<__T> #trait_name for (__T,) {}
            });

            type_state_params.push(quote! { #param: #trait_name });
        } else {
            type_state_params.push(quote! { #param });
        }
    }

    let builder_type_args = if type_args.is_empty() {
        quote! { (#(#type_state_idents,)*) }
    } else {
        quote! { #(#type_args,)* (#(#type_state_idents,)*) }
    };

    let impl_params = if generic_params.is_empty() {
        quote! { #(#type_state_params),* }
    } else {
        quote! {
            #(#generic_params,)* #(#type_state_params),*
        }
    };

    let module_items = quote! {
        #[doc(hidden)]
        pub trait __CheckAllRequired {}

        #[doc(hidden)]
        pub trait __CheckMissing {
            fn __check_missing(self) -> Self;
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        pub fn __require_props<__B: __CheckAllRequired>(
            _: &__B,
        ) {
        }
    };

    let check_all_required_impl = quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#impl_params>
            #module_name::__CheckAllRequired
            for #builder_name<#builder_type_args>
        #where_clause
        {}
    };

    let check_missing_impl = quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#impl_params>
            #module_name::__CheckMissing
            for #builder_name<#builder_type_args>
        #where_clause
        {
            fn __check_missing(self) -> Self { self }
        }
    };

    ModuleRequiredCheckTokens {
        marker_traits: quote! { #(#marker_traits)* },
        module_items,
        check_all_required_impl,
        check_missing_impl,
    }
}

// -------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::{parse_quote, ItemFn};

    fn parse_ty(s: &str) -> Type {
        syn::parse_str(s).unwrap()
    }

    fn ident(s: &str) -> Ident {
        Ident::new(s, Span::call_site())
    }

    /// Parse generics (including where clause) by wrapping in a
    /// function signature, since `syn::Generics` alone doesn't
    /// parse where clauses.
    fn parse_generics(s: &str) -> syn::Generics {
        let (angle_part, where_part) = if let Some(idx) = s.find("where") {
            (&s[..idx], &s[idx..])
        } else {
            (s, "")
        };
        let fn_str = format!("fn f{angle_part}() {where_part} {{}}");
        let item_fn: ItemFn = syn::parse_str(&fn_str).unwrap();
        item_fn.sig.generics
    }

    // --- type_contains_ident ---

    #[test]
    fn type_contains_ident_bare() {
        assert!(type_contains_ident(&parse_ty("F"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_different() {
        assert!(!type_contains_ident(&parse_ty("F"), &ident("G")));
    }

    #[test]
    fn type_contains_ident_in_vec() {
        assert!(type_contains_ident(&parse_ty("Vec<F>"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_nested() {
        assert!(type_contains_ident(
            &parse_ty("Option<Vec<F>>"),
            &ident("F")
        ));
    }

    #[test]
    fn type_contains_ident_absent() {
        assert!(!type_contains_ident(&parse_ty("Vec<i32>"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_in_hashmap() {
        assert!(type_contains_ident(
            &parse_ty("std::collections::HashMap<String, F>"),
            &ident("F")
        ));
    }

    // --- is_bare_generic_param ---

    #[test]
    fn bare_generic_exact() {
        assert!(is_bare_generic_param(&parse_ty("F"), &ident("F")));
    }

    #[test]
    fn bare_generic_wrapped() {
        assert!(!is_bare_generic_param(&parse_ty("Vec<F>"), &ident("F")));
    }

    #[test]
    fn bare_generic_different_name() {
        assert!(!is_bare_generic_param(&parse_ty("G"), &ident("F")));
    }

    #[test]
    fn bare_generic_option() {
        assert!(!is_bare_generic_param(&parse_ty("Option<F>"), &ident("F")));
    }

    // --- collect_predicates_for_param ---

    #[test]
    fn collect_inline_bound() {
        let generics: syn::Generics = parse_quote! { <F: Clone> };
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_where_clause_bound() {
        let generics = parse_generics("<F> where F: Fn() -> bool");
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_inline_and_where() {
        let generics = parse_generics("<F: Clone> where F: Send");
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn collect_different_param() {
        let generics = parse_generics("<F: Clone> where G: Send");
        let preds = collect_predicates_for_param(&generics, &ident("G"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_no_bounds() {
        let generics: syn::Generics = parse_quote! { <F> };
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 0);
    }

    // --- param_needs_structural_bounds ---

    #[test]
    fn structural_bare_generic_no() {
        let ty = parse_ty("F");
        assert!(!param_needs_structural_bounds(&ident("F"), &[&ty]));
    }

    #[test]
    fn structural_wrapped_generic_yes() {
        let ty = parse_ty("Vec<F>");
        assert!(param_needs_structural_bounds(&ident("F"), &[&ty]));
    }

    #[test]
    fn structural_concrete_no() {
        let ty = parse_ty("i32");
        assert!(!param_needs_structural_bounds(&ident("F"), &[&ty]));
    }

    #[test]
    fn structural_inside_wrapper_yes() {
        let ty = parse_ty("ServerAction<F>");
        assert!(param_needs_structural_bounds(&ident("F"), &[&ty]));
    }

    // --- strip_non_structural_bounds ---

    #[test]
    fn strip_bare_generic() {
        let generics = parse_generics("<F: Fn()> where F: Fn()");
        let ty = parse_ty("F");
        let stripped = strip_non_structural_bounds(&generics, &[&ty]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty());
        } else {
            panic!("expected type param");
        }
        assert!(stripped.where_clause.is_none());
    }

    #[test]
    fn strip_keeps_structural() {
        let generics = parse_generics("<F: Clone> where F: Clone");
        let ty = parse_ty("Vec<F>");
        let stripped = strip_non_structural_bounds(&generics, &[&ty]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(!tp.bounds.is_empty());
        } else {
            panic!("expected type param");
        }
        assert!(stripped.where_clause.is_some());
    }

    #[test]
    fn strip_mixed() {
        let generics = parse_generics("<F: Fn(), S: Clone> where S: Clone");
        let ty_f = parse_ty("F");
        let ty_s = parse_ty("ServerAction<S>");
        let stripped = strip_non_structural_bounds(&generics, &[&ty_f, &ty_s]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty(), "F bounds should be stripped");
        }
        if let GenericParam::Type(tp) = &stripped.params[1] {
            assert!(!tp.bounds.is_empty(), "S bounds should be kept");
        }
        let wc = stripped.where_clause.as_ref().unwrap();
        assert_eq!(wc.predicates.len(), 1);
        let pred_str = wc.predicates[0].to_token_stream().to_string();
        assert!(
            pred_str.contains("S"),
            "kept predicate should be for S, got: {pred_str}"
        );
    }
}
