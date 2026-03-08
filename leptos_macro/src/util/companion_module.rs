use super::{
    children::is_children_prop,
    prop_checks::{generate_prop_checks, PropCheckOutput},
    strip_raw_prefix, type_analysis, PropLike,
};
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

/// Configuration for generating companion module internals.
///
/// Captures the small differences between components and slots so
/// that [`generate_companion_internals`] can run the shared
/// computation sequence once.
pub(crate) struct CompanionConfig<'a, P: PropLike> {
    /// Full generics from the original function/struct (all bounds).
    pub original_generics: &'a syn::Generics,
    /// Generics with behavioral bounds stripped (structural only).
    pub stripped_generics: &'a syn::Generics,
    /// Name of the companion module (e.g. `ComponentName` for
    /// components, `__SlotName` for slots). Used for trait impl
    /// paths.
    pub module_name: &'a Ident,
    /// Human-readable name for error messages (e.g. `ComponentName`,
    /// `SlotName`).
    pub display_name: &'a Ident,
    /// `"component"` or `"slot"`.
    pub kind: &'a str,
    /// Name of the props struct (e.g. `ComponentNameProps` for
    /// components, `SlotName` for slots).
    pub props_name: &'a Ident,
    /// Props for this component/slot.
    pub props: &'a [P],
}

/// Assembled companion module body, ready for embedding.
///
/// Produced by [`generate_companion_internals`] and consumed by
/// `component.rs` / `slot.rs` when assembling the final `quote!`
/// template.
pub(crate) struct CompanionModuleBody {
    /// Shared items for inside the companion module: marker traits,
    /// module builder, check traits/wrappers, prop presence tracker,
    /// Helper struct and its impl blocks.
    pub module_items: TokenStream,
    /// Trait implementations for outside the companion module.
    pub trait_impls: TokenStream,
    /// Helper constructor argument (needed by the accessor
    /// fn/method).
    pub helper_constructor_arg: TokenStream,
}

/// Runs the shared companion-module computation sequence and
/// assembles the shared template.
///
/// Both `#[component]` and `#[slot]` call this instead of invoking
/// the individual generation functions. The returned
/// [`CompanionModuleBody`] contains the assembled `module_items`
/// token stream (ready to embed inside the companion module) plus
/// `trait_impls` (for outside the module) and the
/// `helper_constructor_arg` (needed by the caller's accessor
/// fn/method).
pub(crate) fn generate_companion_internals<P: PropLike>(
    config: &CompanionConfig<'_, P>,
) -> CompanionModuleBody {
    let field_types: Vec<_> = config.props.iter().map(|p| p.ty()).collect();

    let PropCheckOutput {
        check_traits,
        trait_impls,
        wrapper_items,
        unbounded_helper_methods,
        bounded_helper_methods,
    } = generate_prop_checks(
        config.original_generics,
        config.module_name,
        config.display_name,
        config.kind,
        config.props,
        &field_types,
    );

    let PresenceOutput {
        items: presence_items,
        initial_return_type: presence_return_type,
    } = generate_presence_and_required_checks(
        config.display_name,
        config.kind,
        config.props,
    );

    let props_name = config.props_name;
    let (stripped_impl, stripped_ty, stripped_where) =
        config.stripped_generics.split_for_impl();
    let (_, orig_ty, _) = config.original_generics.split_for_impl();

    let (builder_fn, builder_ret) = if config.props.is_empty() {
        let ret = quote! { ::leptos::component::EmptyPropsBuilder };
        let func = quote! {
            /// Creates a builder for this component's props.
            pub fn builder() -> #ret {
                ::leptos::component::EmptyPropsBuilder {}
            }
        };
        (func, ret)
    } else {
        let ret = quote! {
            <#props_name #stripped_ty
                as ::leptos::component::Props>::Builder
        };
        let func = quote! {
            /// Creates a builder for this component's props.
            pub fn builder #stripped_impl ()
                -> #ret
                #stripped_where
            {
                <#props_name #stripped_ty
                    as ::leptos::component::Props>::builder()
            }
        };
        (func, ret)
    };

    let (helper_phantom_field, helper_constructor_arg) =
        generate_phantom_data(config.stripped_generics);

    // Bounded impl block for DependentBounds helper methods
    let bounded_helper_impl = if bounded_helper_methods.is_empty() {
        quote! {}
    } else {
        let all_preds =
            type_analysis::collect_all_predicates(config.original_generics);

        quote! {
            #[doc(hidden)]
            #[allow(non_snake_case)]
            impl #stripped_impl Helper #orig_ty
            where #(#all_preds,)*
            {
                #(#bounded_helper_methods)*
            }
        }
    };

    let module_items = quote! {
        #presence_items

        #builder_fn
        #(#check_traits)*
        #(#wrapper_items)*

        // Helper struct — routes all view-macro calls through a
        // single object so generic params share one type variable
        // (constrained by the builder chain).
        #[doc(hidden)]
        pub struct Helper #stripped_impl (
            #helper_phantom_field
        ) #stripped_where;

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl #stripped_impl Helper #orig_ty
            #stripped_where
        {
            #[doc(hidden)]
            pub fn builder(&self) -> #builder_ret {
                builder()
            }

            #[doc(hidden)]
            pub fn presence(&self) -> #presence_return_type {
                presence()
            }

            #(#unbounded_helper_methods)*
        }

        #bounded_helper_impl
    };

    CompanionModuleBody {
        module_items,
        trait_impls: quote! { #(#trait_impls)* },
        helper_constructor_arg,
    }
}

/// Output from [`generate_presence_and_required_checks`].
struct PresenceOutput {
    /// Token stream containing marker traits, `PropPresence`
    /// struct, `presence()` function, and impl blocks.
    items: TokenStream,

    /// The initial return type of `presence()`, e.g.
    /// `PropPresence<(Absent, Absent, ...)>`.
    initial_return_type: TokenStream,
}

/// Generates marker traits for required-prop checking and the
/// presence-tracking builder in a single pass.
///
/// Each required field gets a marker trait with `on_unimplemented`
/// that is implemented for `Present` (but not `Absent`). The
/// presence builder tracks which props are present via type-state
/// (PhantomData tuples) and uses these marker traits for its
/// `require_props` where-clause.
///
/// Two inherent impl blocks on `PropPresence`:
///
/// 1. **Unbounded**: setters + `require_props` (where-clause produces E0277 via
///    marker trait `on_unimplemented` when required props are missing).
///
/// 2. **Bounded**: `check_missing` is only available when all required fields'
///    marker traits are satisfied (presence slots are `Present`). When bounds
///    fail → E0599 → `{error}` type → downstream `.build()` errors absorbed.
///
/// # Parameters
///
/// - `display_name`: human-readable name for error messages
/// - `kind`: `"component"` or `"slot"`
/// - `props`: the component/slot props
fn generate_presence_and_required_checks<P: PropLike>(
    display_name: &Ident,
    kind: &str,
    props: &[P],
) -> PresenceOutput {
    let n = props.len();

    // Collect required-prop names for the note message.
    let names_of_required_props = props
        .iter()
        .filter(|p| p.is_required())
        .map(|p| format!("`{}`", p.name()))
        .join(", ");

    // Generate marker traits and collect trait names for the
    // presence builder bounds in the same loop.
    let mut marker_traits = vec![];
    let type_state_idents: Vec<Ident> =
        (0..n).map(|i| format_ident!("__F{}", i)).collect();

    let mut require_bounds = Vec::new();
    let mut type_state_params = Vec::new();

    for (i, prop) in props.iter().enumerate() {
        let name = prop.name();
        let required = prop.is_required();
        let ty = prop.ty();
        let param = &type_state_idents[i];
        if required {
            let clean_name = strip_raw_prefix(name);
            let trait_name = Ident::new(
                &format!("required_{display_name}_{clean_name}"),
                Span::call_site(),
            );

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

                #[diagnostic::do_not_recommend]
                impl #trait_name for Present {}
            });

            let bound = quote! { #param: #trait_name };
            require_bounds.push(bound.clone());
            type_state_params.push(bound);
        } else {
            type_state_params.push(quote! { #param });
        }
    }

    // Initial type state: all Absent
    let initial_types: Vec<TokenStream> =
        (0..n).map(|_| quote! { Absent }).collect();

    let initial_return_type = if n == 0 {
        quote! { PropPresence<()> }
    } else {
        quote! { PropPresence<(#(#initial_types,)*)> }
    };

    // Setter methods: each one transitions its slot from __Fi to
    // Present
    let setter_methods: Vec<TokenStream> = props
        .iter()
        .enumerate()
        .map(|(i, prop)| {
            let clean = strip_raw_prefix(prop.name());
            let setter_name = Ident::new_raw(&clean, Span::call_site());

            let return_types: Vec<TokenStream> = (0..n)
                .map(|j| {
                    if j == i {
                        quote! { Present }
                    } else {
                        let param = &type_state_idents[j];
                        quote! { #param }
                    }
                })
                .collect();

            quote! {
                pub fn #setter_name(self)
                    -> PropPresence<(#(#return_types,)*)>
                {
                    PropPresence(::core::marker::PhantomData)
                }
            }
        })
        .collect();

    let items = quote! {
        /// Presence type-state marker: prop has NOT been provided.
        #[doc(hidden)]
        pub struct Absent;

        /// Presence type-state marker: prop has been provided.
        #[doc(hidden)]
        pub struct Present;

        #(#marker_traits)*

        #[doc(hidden)]
        pub struct PropPresence<S>(::core::marker::PhantomData<S>);

        #[doc(hidden)]
        pub fn presence() -> #initial_return_type
        {
            PropPresence(::core::marker::PhantomData)
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_idents),*> PropPresence<(#(#type_state_idents,)*)>
        {
            #(#setter_methods)*

            pub fn require_props(&self)
            where
                #(#require_bounds,)*
            {}
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_params),*> PropPresence<(#(#type_state_idents,)*)>
        {
            pub fn check_missing<__B>(
                self,
                builder: __B,
            ) -> __B {
                builder
            }
        }
    };

    PresenceOutput {
        items,
        initial_return_type,
    }
}

/// Generates the `PhantomData` field and constructor argument for the
/// `Helper` struct.
///
/// Returns `(phantom_field, constructor_arg)`. When there are no
/// generic params both are empty token streams.
fn generate_phantom_data(
    generics: &syn::Generics,
) -> (TokenStream, TokenStream) {
    let lifetime_refs: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Lifetime(lt) => Some(&lt.lifetime),
            _ => None,
        })
        .collect();
    let type_idents: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(&tp.ident),
            _ => None,
        })
        .collect();

    if type_idents.is_empty() && lifetime_refs.is_empty() {
        return (quote! {}, quote! {});
    }

    (
        quote! {
            pub ::core::marker::PhantomData<
                fn(#(&#lifetime_refs (),)*) -> (#(#type_idents,)*)
            >
        },
        quote! {
            #[allow(clippy::default_constructed_unit_structs)]
            ::core::marker::PhantomData::default()
        },
    )
}
