use super::{
    children::is_children_prop, strip_raw_prefix, type_analysis, PropLike,
};
use itertools::Itertools;
use proc_macro2::{Group, Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{GenericParam, Generics, Type};

/// Configuration for generating companion module internals.
///
/// Captures the small differences between components and slots so
/// that [`generate_companion_internals`] can run the shared
/// computation sequence once.
pub(crate) struct CompanionConfig<'a, P: PropLike> {
    /// Full generics from the original function/struct (all bounds).
    pub original_generics: &'a Generics,
    /// Generics with behavioral bounds stripped (structural only).
    pub stripped_generics: &'a Generics,
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
    } = generate_prop_presence(config.display_name, config.kind, config.props);

    let props_name = config.props_name;
    let (stripped_impl, stripped_ty, stripped_where) =
        config.stripped_generics.split_for_impl();
    let (_, orig_ty, _) = config.original_generics.split_for_impl();

    let (builder_fn, builder_ret) = if config.props.is_empty() {
        let ret = quote! { ::leptos::component::EmptyPropsBuilder };
        let func = quote! {
            /// Creates a builder for this component's props.
            #[inline(always)]
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
            #[inline(always)]
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
            #[inline(always)]
            pub fn builder(&self) -> #builder_ret {
                builder()
            }

            #[doc(hidden)]
            #[inline(always)]
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

/// Output from [`generate_prop_presence`].
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
fn generate_prop_presence<P: PropLike>(
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
                #[inline(always)]
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
        #[inline(always)]
        pub fn presence() -> #initial_return_type
        {
            PropPresence(::core::marker::PhantomData)
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_idents),*> PropPresence<(#(#type_state_idents,)*)>
        {
            #(#setter_methods)*

            #[inline(always)]
            pub fn require_props(&self)
            where
                #(#require_bounds,)*
            {}
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#(#type_state_params),*> PropPresence<(#(#type_state_idents,)*)>
        {
            #[inline(always)]
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
fn generate_phantom_data(generics: &Generics) -> (TokenStream, TokenStream) {
    let lifetime_refs: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Lifetime(lt) => Some(&lt.lifetime),
            _ => None,
        })
        .collect();
    let type_idents: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(tp) => Some(&tp.ident),
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

/// The generated token streams for per-prop companion checks.
#[derive(Default)]
struct PropCheckOutput {
    /// Trait definitions for inside the companion module.
    check_traits: Vec<TokenStream>,
    /// Trait implementations for outside the companion module.
    trait_impls: Vec<TokenStream>,
    /// Wrapper struct definitions and their impl blocks for inside the
    /// companion module.
    wrapper_items: Vec<TokenStream>,
    /// Check/wrap methods for the `Helper` struct's unbounded impl
    /// block. These take `&self` and use module-local paths (no module
    /// prefix) since they live inside the companion module.
    unbounded_helper_methods: Vec<TokenStream>,
    /// Check/wrap methods that need a bounded impl block on
    /// `Helper` for closure parameter inference. These are
    /// `PropClassification::DependentBounds` methods whose prop type
    /// is a generic type parameter with bounds that reference other
    /// generic params. They all share ONE bounded impl block with ALL
    /// the original where-clause predicates, so cross-parameter
    /// inference works (e.g. `T` gets resolved from `each`'s
    /// `IF: Fn() -> I`, `I: IntoIterator<Item = T>` and is then
    /// available for `key`'s `KF: Fn(&T) -> K`).
    bounded_helper_methods: Vec<TokenStream>,
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
fn generate_prop_checks<'a, P: PropLike>(
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
                        #[inline(always)]
                        pub fn extract_value(self) -> __T { self.0 }
                    }
                });

                // Unbounded check+wrap helper method. The trait bound
                // is satisfied by supertraits on Check_*.
                output.unbounded_helper_methods.push(quote! {
                    #[doc(hidden)]
                    #[inline(always)]
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
                    #[inline(always)]
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
                    #[inline(always)]
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
            #[inline(always)]
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
