//! Shared utility functions for the `#[component]` and `#[slot]`
//! macros.
//!
//! These helpers are used by both `component.rs` and `slot.rs` to
//! generate per-prop type checks, required-prop checks, and phantom
//! fields — all part of the localized error reporting machinery.

pub mod children;
pub mod documentation;
pub mod property_documentation;
pub mod type_analysis;
pub mod typed_builder_opts;

use crate::util::children::is_children_prop;
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{GenericParam, Type, TypePath};

/// Strips the raw identifier prefix (`r#`) from a prop name.
pub(crate) fn clean_prop_name(ident: &Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").unwrap_or(&s).to_owned()
}

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
        .filter(|ident| {
            !type_analysis::param_needs_structural_bounds(ident, field_types)
        })
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
        .find(|ident| type_analysis::is_bare_generic_param(prop_ty, ident))
        .and_then(|param_ident| {
            let preds = type_analysis::collect_predicates_for_param(
                full_generics,
                param_ident,
            );
            if !preds.is_empty()
                && !type_analysis::bounds_reference_other_params(
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
        let bounds = type_analysis::predicates_to_bounds(&param_predicates);
        let bounds_note = bounds.to_string();
        let message = format!(
            "`{{Self}}` is not a valid type for prop `{clean_name}` on {kind} \
             `{display_name}`",
        );
        let hint =
            if type_analysis::predicates_contain_fn_bound(&param_predicates) {
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

                // Outside module: bounded impl.
                //
                // NOTE: `#[diagnostic::do_not_recommend]` was tested
                // here to suppress the noisy E0599 from `.__pass_*()`.
                // However, since the same impl serves BOTH the UFCS
                // (clean E0277) and method (E0599) paths,
                // `do_not_recommend` degrades the E0277 message — it
                // shows `__Check_*` instead of the underlying bound
                // (e.g. `Greetable`). Not worth the trade-off.
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
/// - `fields`: `(name, is_required, type)` for each field
pub(crate) fn generate_module_required_check(
    display_name: &Ident,
    kind: &str,
    fields: &[(&Ident, bool, &Type)],
) -> ModuleRequiredCheckTokens {
    if fields.is_empty() {
        return ModuleRequiredCheckTokens {
            marker_traits: quote! {},
        };
    }

    let names_of_required_props = fields
        .iter()
        .filter(|(_, required, _)| *required)
        .map(|(name, _, _)| format!("`{}`", name.to_string()))
        .join(", ");

    let mut marker_traits = vec![];

    for (name, required, ty) in fields.iter() {
        if *required {
            let clean_name = clean_prop_name(name);
            let trait_name = required_marker_trait_name(display_name, name);

            let (message, label, note) = if is_children_prop(name, ty) {
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

/// Unwraps `Option<T>` to `T`. Aborts if the type is not
/// `Option<T>`.
pub(crate) fn unwrap_option(ty: &Type) -> Type {
    const STD_OPTION_MSG: &str =
        "make sure you're not shadowing the `std::option::Option` type that \
         is automatically imported from the standard prelude";

    if let Type::Path(TypePath {
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
pub(crate) fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath {
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
/// - `fields`: `(name, is_required, type)` for each field (same as
///   `generate_module_required_check`)
pub(crate) fn generate_module_presence_check(
    module_name: &Ident,
    display_name: &Ident,
    fields: &[(&Ident, bool, &Type)],
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
                impl
                    #module_name::__CheckPresence
                    for #module_name::__PresenceBuilder<()>
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
        .map(|(i, (name, _required, _ty))| {
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
        .map(|(i, (name, required, _ty))| {
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
