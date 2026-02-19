//! Shared utility functions for the `#[component]` and `#[slot]`
//! macros.
//!
//! These helpers are used by both `component.rs` and `slot.rs` to
//! generate per-prop type checks, required-prop checks, and phantom
//! fields — all part of the localized error reporting machinery.

use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{visit::Visit, GenericParam, Type, TypePath, WherePredicate};

// -------------------------------------------------------------------
// Small helpers
// -------------------------------------------------------------------

/// Strips the raw identifier prefix (`r#`) from a prop name.
pub(crate) fn clean_prop_name(ident: &Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").unwrap_or(&s).to_owned()
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
    kind: &str,
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
            "`{{Self}}` is not a valid type for prop `{clean_name}` on {kind} \
             `{display_name}`",
        );
        let bounds_str = bounds_note.trim_start();
        let hint = if bounds_str.starts_with("Fn")
            || bounds_str.starts_with("FnMut")
            || bounds_str.starts_with("FnOnce")
        {
            " — a closure or function reference"
        } else {
            ""
        };
        let note = format!("required: `{bounds_note}`{hint}");

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
///   `on_unimplemented`, `__check_*(&self)` and `__pass_*(self)`)
///   inside module; bounded impl outside.
///   View macro calls UFCS:
///   `<_ as Module::__Check_foo>::__check_foo(&value)` — E0277
///   with `on_unimplemented` (works for all types including
///   closures), then method: `value.__pass_foo()` — E0599
///   produces `{error}` for downstream suppression.
///
///   NOTE: Two steps are needed because E0599 doesn't show
///   `on_unimplemented` for anonymous types (closures). If rustc
///   gains that capability, the UFCS step could be dropped to
///   reduce bounded-generic prop errors from 2 to 1.
/// - **Everything else**: blanket `__Check_*` trait inside module
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
    kind: &str,
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
            kind,
            &preamble,
            full_generics,
        );

        // Intentionally call_site span: these synthetic idents should
        // not link back to any user source location.
        let check_trait_name = format_ident!("__Check_{}", clean_name);
        let check_method_name = format_ident!("__check_{}", clean_name);
        let pass_method_name = format_ident!("__pass_{}", clean_name);

        match classification {
            PropClassification::BoundedSingleParam {
                bounds,
                message,
                note,
            } => {
                // Inside module: single trait with both check
                // (UFCS, E0277) and pass (method, E0599 → {error})
                module_check_traits.push(quote! {
                    #[doc(hidden)]
                    #[diagnostic::on_unimplemented(
                        message = #message,
                        note = #note
                    )]
                    #[allow(non_camel_case_types)]
                    pub trait #check_trait_name {
                        fn #check_method_name(&self);
                        fn #pass_method_name(self) -> Self;
                    }
                });

                // Outside module: bounded impl
                check_trait_impls.push(quote! {
                    #[doc(hidden)]
                    impl<__T: #bounds>
                        #module_name::#check_trait_name for __T
                    {
                        fn #check_method_name(&self) {}
                        fn #pass_method_name(self) -> Self {
                            self
                        }
                    }
                });
            }
            PropClassification::PassThrough => {
                // Blanket check trait — all types pass.
                module_check_traits.push(quote! {
                    #[doc(hidden)]
                    #[allow(non_camel_case_types)]
                    pub trait #check_trait_name {
                        fn #check_method_name(&self);
                        fn #pass_method_name(self) -> Self;
                    }
                });

                check_trait_impls.push(quote! {
                    #[doc(hidden)]
                    impl<__T> #module_name::#check_trait_name
                        for __T
                    {
                        fn #check_method_name(&self) {}
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
// Shared helper for marker trait names
// -------------------------------------------------------------------

/// Returns the marker trait name for a required field, e.g.
/// `__required_Inner_concrete_i32`. Used by both
/// `generate_module_required_check` and
/// `generate_module_presence_check` to ensure consistent naming.
pub(crate) fn required_marker_trait_name(
    display_name: &Ident,
    prop_name: &Ident,
) -> Ident {
    let clean_name = clean_prop_name(prop_name);
    Ident::new(
        &format!("__required_{display_name}_{clean_name}"),
        Span::call_site(),
    )
}

// -------------------------------------------------------------------
// Module-based required check generation
// -------------------------------------------------------------------

/// The output of `generate_module_required_check`.
pub(crate) struct ModuleRequiredCheckTokens {
    /// Marker trait definitions (with `on_unimplemented`) at module
    /// scope, outside the companion module. Used by both
    /// `__CheckPresence` (UFCS, clean E0277) and the
    /// `__check_missing` inherent method on `__PresenceBuilder`
    /// (E0599 → `{error}` propagation).
    pub marker_traits: TokenStream,
}

/// Generates marker traits for required-prop checking.
///
/// Each required field gets a marker trait with `on_unimplemented`
/// that is implemented only for 1-tuples `(__T,)`. These markers
/// are used by:
/// - `__CheckPresence` on the presence builder (independent of
///   `{error}` contamination) — produces clean E0277
/// - `__check_missing` inherent method on `__PresenceBuilder` —
///   produces `{error}` for downstream suppression via E0599
///
/// - `display_name`: human-readable name for error messages
/// - `kind`: `"component"` or `"slot"`
/// - `fields`: `(name, is_required)` for each field
pub(crate) fn generate_module_required_check(
    display_name: &Ident,
    kind: &str,
    fields: &[(&Ident, bool)],
) -> ModuleRequiredCheckTokens {
    if fields.is_empty() {
        return ModuleRequiredCheckTokens {
            marker_traits: quote! {},
        };
    }

    let names_of_required_props = fields
        .iter()
        .filter(|(_, required)| *required)
        .map(|(name, _)| format!("`{}`", name.to_string()))
        .join(", ");

    let mut marker_traits = vec![];

    for (name, required) in fields.iter() {
        if *required {
            let clean_name = clean_prop_name(name);
            let trait_name = required_marker_trait_name(display_name, name);

            // TODO: Improve "children" detection.
            let (message, label, note) = if clean_name == "children" {
                (
                    format!("{kind} `{display_name}` requires children"),
                    String::from("children required"),
                    String::from(
                        "add child elements between the opening and closing \
                         tags",
                    ),
                )
            } else {
                (
                    format!(
                        "missing required prop `{clean_name}` on {kind} \
                         `{display_name}`"
                    ),
                    format!("missing prop `{clean_name}`"),
                    format!("required props: [{names_of_required_props}]"),
                )
            };

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
        }
    }

    ModuleRequiredCheckTokens {
        marker_traits: quote! { #(#marker_traits)* },
    }
}

// -------------------------------------------------------------------
// Module builder generation (shared by components and slots)
// -------------------------------------------------------------------

/// Generates the `__builder()` function for inside a companion module.
///
/// For no-props components/slots, returns an `EmptyPropsBuilder`.
/// Otherwise, delegates to the props struct's `Props::builder()`.
pub(crate) fn generate_module_builder(
    no_props: bool,
    stripped_generics: &syn::Generics,
    props_name: &Ident,
) -> TokenStream {
    if no_props {
        quote! {
            #[doc(hidden)]
            pub fn __builder()
                -> ::leptos::component::EmptyPropsBuilder
            {
                ::leptos::component::EmptyPropsBuilder {}
            }
        }
    } else {
        let (impl_generics, ty_generics, where_clause) =
            stripped_generics.split_for_impl();
        quote! {
            #[doc(hidden)]
            pub fn __builder #impl_generics ()
                -> <super::#props_name #ty_generics
                    as ::leptos::component::Props>::Builder
                #where_clause
            {
                <super::#props_name #ty_generics
                    as ::leptos::component::Props>::builder()
            }
        }
    }
}

// -------------------------------------------------------------------
// TypedBuilder attribute generation (shared by components and slots)
// -------------------------------------------------------------------

/// Options for generating `#[builder(...)]` attributes on prop fields.
///
/// Used by both components and slots to produce the correct
/// TypedBuilder annotations for each prop.
pub(crate) struct TypedBuilderOpts<'a> {
    pub default: bool,
    pub default_with_value: Option<syn::Expr>,
    pub strip_option: bool,
    pub into: bool,
    pub ty: &'a syn::Type,
}

impl TypedBuilderOpts<'_> {
    /// Generates `#[serde(...)]` attributes matching the builder
    /// defaults. Only used by component props serialization.
    pub fn to_serde_tokens(&self) -> TokenStream {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        if !default.is_empty() {
            quote! { #[serde(#default)] }
        } else {
            quote! {}
        }
    }
}

impl quote::ToTokens for TypedBuilderOpts<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default_code=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        // If self.strip_option && self.into, then the strip_option
        // will be represented as part of the transform closure.
        let strip_option = if self.strip_option && !self.into {
            quote! { strip_option, }
        } else {
            quote! {}
        };

        let into = if self.into {
            if !self.strip_option {
                let ty = &self.ty;
                quote! {
                    fn transform<__IntoReactiveValueMarker>(value: impl ::leptos::prelude::IntoReactiveValue<#ty, __IntoReactiveValueMarker>) -> #ty {
                        value.into_reactive_value()
                    },
                }
            } else {
                let ty = unwrap_option(self.ty);
                quote! {
                    fn transform<__IntoReactiveValueMarker>(value: impl ::leptos::prelude::IntoReactiveValue<#ty, __IntoReactiveValueMarker>) -> Option<#ty> {
                        Some(value.into_reactive_value())
                    },
                }
            }
        } else {
            quote! {}
        };

        let setter = if !strip_option.is_empty() || !into.is_empty() {
            quote! { setter(#strip_option #into) }
        } else {
            quote! {}
        };

        let output = if !default.is_empty() || !setter.is_empty() {
            quote! { #[builder(#default #setter)] }
        } else {
            quote! {}
        };

        tokens.append_all(output);
    }
}

/// Unwraps `Option<T>` to `T`. Aborts if the type is not
/// `Option<T>`.
pub(crate) fn unwrap_option(ty: &syn::Type) -> syn::Type {
    const STD_OPTION_MSG: &str =
        "make sure you're not shadowing the `std::option::Option` type that \
         is automatically imported from the standard prelude";

    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let [first] = &segments.iter().collect::<Vec<_>>()[..] {
            if first.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(
                    syn::AngleBracketedGenericArguments { args, .. },
                ) = &first.arguments
                {
                    if let [syn::GenericArgument::Type(ty)] =
                        &args.iter().collect::<Vec<_>>()[..]
                    {
                        return ty.clone();
                    }
                }
            }
        }
    }

    proc_macro_error2::abort!(
        ty,
        "`Option` must be `std::option::Option`";
        help = STD_OPTION_MSG
    );
}

/// Returns true if the type is `Option<_>`.
pub(crate) fn is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if let [first] = &segments.iter().collect::<Vec<_>>()[..] {
            first.ident == "Option"
        } else {
            false
        }
    } else {
        false
    }
}

// -------------------------------------------------------------------
// Module-based presence check generation
// -------------------------------------------------------------------

/// The split output of `generate_module_presence_check`.
pub(crate) struct ModulePresenceTokens {
    /// Items inside the companion module: `__PresenceBuilder` struct,
    /// `__presence()` constructor, setter methods, `__CheckPresence`
    /// trait.
    pub module_items: TokenStream,
    /// `impl __CheckPresence for __PresenceBuilder` outside module.
    pub check_presence_impl: TokenStream,
}

/// Generates a lightweight presence-tracking builder that is
/// independent of actual prop values (and thus immune to `{error}`
/// type contamination from wrong-type props).
///
/// The presence builder tracks which props are present via
/// type-state (PhantomData tuples). Its `__require_props` (via
/// `__CheckPresence`) fires E0277 for missing required props
/// regardless of whether the real builder is contaminated by
/// `{error}`.
///
/// - `module_name`: name of the companion module
/// - `display_name`: human-readable name for error messages
/// - `fields`: `(name, is_required)` for each field (same as
///   `generate_module_required_check`)
pub(crate) fn generate_module_presence_check(
    module_name: &Ident,
    display_name: &Ident,
    fields: &[(&Ident, bool)],
) -> ModulePresenceTokens {
    if fields.is_empty() {
        return ModulePresenceTokens {
            module_items: quote! {
                #[doc(hidden)]
                pub struct __PresenceBuilder<S>(
                    ::core::marker::PhantomData<S>,
                );

                #[doc(hidden)]
                pub fn __presence() -> __PresenceBuilder<()> {
                    __PresenceBuilder(::core::marker::PhantomData)
                }

                #[doc(hidden)]
                impl<S> __PresenceBuilder<S> {
                    pub fn __check_missing<__B>(
                        self,
                        builder: __B,
                    ) -> __B {
                        builder
                    }
                }

                #[doc(hidden)]
                pub trait __CheckPresence {
                    fn __require_props(&self);
                }
            },
            check_presence_impl: quote! {
                impl<__T>
                    #module_name::__CheckPresence for __T
                {
                    fn __require_props(&self) {}
                }
            },
        };
    }

    let n = fields.len();
    let type_state_idents: Vec<Ident> =
        (0..n).map(|i| format_ident!("__F{}", i)).collect();

    // Initial type state: all ()
    let initial_types: Vec<TokenStream> =
        (0..n).map(|_| quote! { () }).collect();

    // Setter methods: each one transitions its slot from __Fi to ((),)
    let setter_methods: Vec<TokenStream> = fields
        .iter()
        .enumerate()
        .map(|(i, (name, _required))| {
            let clean = clean_prop_name(name);
            let setter_name = Ident::new_raw(&clean, Span::call_site());

            let return_types: Vec<TokenStream> = (0..n)
                .map(|j| {
                    if j == i {
                        quote! { ((),) }
                    } else {
                        let param = &type_state_idents[j];
                        quote! { #param }
                    }
                })
                .collect();

            quote! {
                pub fn #setter_name(self)
                    -> __PresenceBuilder<(#(#return_types,)*)>
                {
                    __PresenceBuilder(::core::marker::PhantomData)
                }
            }
        })
        .collect();

    // Type-state params for __CheckPresence impl: required fields
    // get marker trait bounds, optional fields are unconstrained.
    let type_state_params: Vec<TokenStream> = fields
        .iter()
        .enumerate()
        .map(|(i, (name, required))| {
            let param = &type_state_idents[i];
            if *required {
                let trait_name = required_marker_trait_name(display_name, name);
                quote! { #param: #trait_name }
            } else {
                quote! { #param }
            }
        })
        .collect();

    let module_items = quote! {
        #[doc(hidden)]
        pub struct __PresenceBuilder<S>(
            ::core::marker::PhantomData<S>,
        );

        #[doc(hidden)]
        pub fn __presence()
            -> __PresenceBuilder<(#(#initial_types,)*)>
        {
            __PresenceBuilder(::core::marker::PhantomData)
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_idents),*>
            __PresenceBuilder<(#(#type_state_idents,)*)>
        {
            #(#setter_methods)*
        }

        // Bounded inherent impl: `__check_missing` is only
        // available when all required fields' marker traits are
        // satisfied (i.e. their presence slots are `(T,)`, not
        // `()`). When bounds fail → E0599 → `{error}` type →
        // downstream `.build()` errors absorbed.
        //
        // This is an inherent method on `__PresenceBuilder`,
        // NOT a trait, so there is no ambiguity when multiple
        // component modules are in scope.
        //
        // Why `__check_missing` receives the typed builder:
        //
        // TypedBuilder enforces required fields for direct
        // builder usage (`Props::builder().field(val).build()`).
        // When the view macro omits a required prop, the builder
        // setter is never called, so TypedBuilder's `.build()`
        // fails with confusing internal errors (E0061,
        // deprecation warnings with names like
        // `PropsBuilder_Error_Missing_required_field_foo`).
        //
        // `__check_missing` suppresses these by converting the
        // builder to `{error}` type (via E0599) when presence
        // bounds fail. `__require_props` (E0277) already
        // provides the clean error. The `{error}` builder
        // absorbs `.build()` and all downstream errors.
        //
        // Making all TypedBuilder fields have defaults
        // (`unreachable!()`) would eliminate the need for this,
        // but would break direct builder usage — required fields
        // would panic at runtime instead of failing at compile
        // time.
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_params),*>
            __PresenceBuilder<(#(#type_state_idents,)*)>
        {
            pub fn __check_missing<__B>(
                self,
                builder: __B,
            ) -> __B {
                builder
            }
        }

        #[doc(hidden)]
        pub trait __CheckPresence {
            fn __require_props(&self);
        }
    };

    let check_presence_impl = quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_params),*>
            #module_name::__CheckPresence
            for #module_name::__PresenceBuilder<(
                #(#type_state_idents,)*
            )>
        {
            fn __require_props(&self) {}
        }
    };

    ModulePresenceTokens {
        module_items,
        check_presence_impl,
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
