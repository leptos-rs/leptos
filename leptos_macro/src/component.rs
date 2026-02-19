use crate::util::{
    collect_phantom_type_params, generate_module_builder,
    generate_module_checks, generate_module_presence_check,
    generate_module_required_check, generate_phantom_field, generate_prop_docs,
    prop_to_doc, strip_non_structural_bounds, typed_builder_opts,
    ModuleCheckTokens, ModulePresenceTokens, ModuleRequiredCheckTokens,
    PropDocInput, PropDocStyle,
};
use attribute_derive::FromAttr;
use convert_case::{
    Case::{Pascal, Snake},
    Casing,
};
use convert_case_extras::is_case;
use itertools::Itertools;
use leptos_hot_reload::parsing::value_to_string;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error2::abort;
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use std::hash::DefaultHasher;
use syn::{
    parse::Parse, parse_quote, spanned::Spanned, token::Colon,
    visit_mut::VisitMut, Attribute, FnArg, GenericParam, Item, ItemFn, LitStr,
    Meta, Pat, PatIdent, Path, ReturnType, Signature, Stmt, Type,
    TypeImplTrait, TypeParam, TypePath, Visibility,
};

pub struct Model {
    is_transparent: bool,
    is_lazy: bool,
    island: Option<String>,
    docs: Docs,
    unknown_attrs: UnknownAttrs,
    vis: Visibility,
    name: Ident,
    props: Vec<Prop>,
    body: ItemFn,
    ret: ReturnType,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut item = ItemFn::parse(input)?;
        maybe_modify_return_type(&mut item.sig.output);

        convert_impl_trait_to_generic(&mut item.sig);

        let docs = Docs::new(&item.attrs);
        let unknown_attrs = UnknownAttrs::new(&item.attrs);

        let props = item
            .sig
            .inputs
            .clone()
            .into_iter()
            .map(Prop::new)
            .collect::<Vec<_>>();

        // We need to remove the `#[doc = ""]` and `#[builder(_)]`
        // attrs from the function signature
        drain_filter(&mut item.attrs, |attr| match &attr.meta {
            Meta::NameValue(attr) => attr.path == parse_quote!(doc),
            Meta::List(attr) => attr.path == parse_quote!(prop),
            _ => false,
        });
        item.sig.inputs.iter_mut().for_each(|arg| {
            if let FnArg::Typed(ty) = arg {
                drain_filter(&mut ty.attrs, |attr| match &attr.meta {
                    Meta::NameValue(attr) => attr.path == parse_quote!(doc),
                    Meta::List(attr) => attr.path == parse_quote!(prop),
                    _ => false,
                });
            }
        });

        Ok(Self {
            is_transparent: false,
            is_lazy: false,
            island: None,
            docs,
            unknown_attrs,
            vis: item.vis.clone(),
            name: convert_from_snake_case(&item.sig.ident),
            props,
            ret: item.sig.output.clone(),
            body: item,
        })
    }
}

/// Exists to fix nested routes defined in a separate component in erased mode,
/// by replacing the return type with AnyNestedRoute, which is what it'll be, but is required as the return type for compiler inference.
fn maybe_modify_return_type(ret: &mut ReturnType) {
    #[cfg(feature = "__internal_erase_components")]
    {
        if let ReturnType::Type(_, ty) = ret {
            if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty.as_ref() {
                // If one of the bounds is MatchNestedRoutes, we need to replace the return type with AnyNestedRoute:
                if bounds.iter().any(|bound| {
                    if let syn::TypeParamBound::Trait(trait_bound) = bound {
                        if trait_bound.path.segments.iter().any(
                            |path_segment| {
                                path_segment.ident == "MatchNestedRoutes"
                            },
                        ) {
                            return true;
                        }
                    }
                    false
                }) {
                    *ty = parse_quote!(
                        ::leptos_router::any_nested_route::AnyNestedRoute
                    );
                }
            }
        }
    }
    #[cfg(not(feature = "__internal_erase_components"))]
    {
        let _ = ret;
    }
}

// implemented manually because Vec::drain_filter is nightly only
// follows std recommended parallel
pub fn drain_filter<T>(
    vec: &mut Vec<T>,
    mut some_predicate: impl FnMut(&mut T) -> bool,
) {
    let mut i = 0;
    while i < vec.len() {
        if some_predicate(&mut vec[i]) {
            _ = vec.remove(i);
        } else {
            i += 1;
        }
    }
}

pub fn convert_from_snake_case(name: &Ident) -> Ident {
    let name_str = name.to_string();
    if !is_case(&name_str, Snake) {
        name.clone()
    } else {
        Ident::new(&name_str.to_case(Pascal), name.span())
    }
}

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            is_transparent,
            is_lazy,
            island,
            docs,
            unknown_attrs,
            vis,
            name,
            props,
            body,
            ret,
        } = self;
        let is_island = island.is_some();

        let no_props = props.is_empty();

        // check for components that end ;
        if !is_transparent {
            let ends_semi =
                body.block.stmts.iter().last().and_then(|stmt| match stmt {
                    Stmt::Item(Item::Macro(mac)) => mac.semi_token.as_ref(),
                    _ => None,
                });
            if let Some(semi) = ends_semi {
                proc_macro_error2::emit_error!(
                    semi.span(),
                    "A component that ends with a `view!` macro followed by a \
                     semicolon will return (), an empty view. This is usually \
                     an accident, not intentional, so we prevent it. If you’d \
                     like to return (), you can do it it explicitly by \
                     returning () as the last item from the component."
                );
            }
        }

        #[allow(clippy::redundant_clone)] // false positive
        let body_name = body.sig.ident.clone();

        let (impl_generics, generics, where_clause) =
            body.sig.generics.split_for_impl();

        // Keep a reference to the full original generics before
        // `body` is shadowed later by a quote! block.
        let original_generics = &body.sig.generics;

        // --- Generic bounds strategy ---
        // Separate structural bounds (needed for field types like
        // `ServerAction<ServFn>`) from behavioral bounds (like
        // `F: Fn() -> bool` for bare generic fields). The struct
        // only carries structural bounds; behavioral bounds are
        // deferred to per-prop check methods for localized error
        // reporting.

        // Struct generics: keep only bounds that are structurally
        // needed by field types (e.g. `ServerAction<ServFn>` needs
        // `ServFn: ServerFn`).
        let field_types: Vec<&Type> = props.iter().map(|p| &p.ty).collect();
        let behavioral_bounds_stripped_generics =
            strip_non_structural_bounds(&body.sig.generics, &field_types);
        let (struct_impl_generics, _, struct_where_clause) =
            behavioral_bounds_stripped_generics.split_for_impl();

        let phantom_type_params =
            collect_phantom_type_params(&body.sig.generics, &field_types);

        let props_name = format_ident!("{name}Props");
        let props_builder_name = format_ident!("{name}PropsBuilder");
        let props_serialized_name = format_ident!("{name}PropsSerialized");
        #[cfg(feature = "tracing")]
        let trace_name = format!("<{name} />");

        let is_island_with_children =
            is_island && props.iter().any(|prop| prop.name.ident == "children");
        let is_island_with_other_props = is_island
            && ((is_island_with_children && props.len() > 1)
                || (!is_island_with_children && !props.is_empty()));

        let prop_builder_fields =
            prop_builder_fields(vis, props, is_island_with_other_props);
        let props_serializer = if is_island_with_other_props {
            let fields = prop_serializer_fields(vis, props);
            quote! {
                #[derive(::leptos::serde::Deserialize)]
                #vis struct #props_serialized_name {
                    #fields
                }
            }
        } else {
            quote! {}
        };

        let prop_names = prop_names(props);

        let builder_name_doc = LitStr::new(
            &format!(" Props for the [`{name}`] component."),
            name.span(),
        );

        let doc_inputs: Vec<PropDocInput> = props
            .iter()
            .map(|p| PropDocInput {
                docs: &p.docs,
                name: &p.name.ident,
                ty: &p.ty,
                is_optional: p.prop_opts.is_optional(),
                optional: p.prop_opts.optional,
                strip_option: p.prop_opts.strip_option,
                into: p.prop_opts.into,
            })
            .collect();
        let component_fn_prop_docs = generate_prop_docs(&doc_inputs);
        let docs_and_prop_docs = if component_fn_prop_docs.is_empty() {
            // Avoid generating an empty doc line in case the component has no doc and no props.
            quote! {
                #docs
            }
        } else {
            quote! {
                #docs
                #[doc = ""]
                #component_fn_prop_docs
            }
        };

        let (
            tracing_instrument_attr,
            tracing_span_expr,
            tracing_guard_expr,
            tracing_props_expr,
        ) = {
            #[cfg(feature = "tracing")]
            {
                /* TODO for 0.8: fix this
                 *
                 * The problem is that cargo now warns about an expected "tracing" cfg if
                 * you don't have a "tracing" feature in your actual crate
                 *
                 * However, until https://github.com/tokio-rs/tracing/pull/1819 is merged
                 * (?), you can't provide an alternate path for `tracing` (for example,
                 * ::leptos::tracing), which means that if you're going to use the macro
                 * you *must* have `tracing` in your Cargo.toml.
                 *
                 * Including the feature-check here causes cargo warnings on
                 * previously-working projects.
                 *
                 * Removing the feature-check here breaks any project that uses leptos with
                 * the tracing feature turned on, but without a tracing dependency in its
                 * Cargo.toml.
                 * /
                 */
                let instrument = cfg!(feature = "trace-components").then(|| quote! {
                    #[cfg_attr(
                        feature = "tracing",
                        ::leptos::tracing::instrument(level = "info", name = #trace_name, skip_all)
                    )]
                });

                (
                    quote! {
                        #[allow(clippy::let_with_type_underscore)]
                        #instrument
                    },
                    quote! {
                        let __span = ::leptos::tracing::Span::current();
                    },
                    quote! {
                        #[cfg(debug_assertions)]
                        let _guard = __span.entered();
                    },
                    if no_props || !cfg!(feature = "trace-component-props") {
                        quote!()
                    } else {
                        quote! {
                            ::leptos::leptos_dom::tracing_props![#prop_names];
                        }
                    },
                )
            }

            #[cfg(not(feature = "tracing"))]
            {
                (quote!(), quote!(), quote!(), quote!())
            }
        };

        let component_id = name.to_string();
        let hydrate_fn_name = is_island.then(|| {
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            island.hash(&mut hasher);
            let caller = hasher.finish() as usize;
            Ident::new(&format!("{component_id}_{caller:?}"), name.span())
        });

        let island_serialize_props = if is_island_with_other_props {
            quote! {
                let _leptos_ser_props = ::leptos::serde_json::to_string(&props).expect("couldn't serialize island props");
            }
        } else {
            quote! {}
        };
        let island_serialized_props = if is_island_with_other_props {
            quote! {
                .with_props( _leptos_ser_props)
            }
        } else {
            quote! {}
        };

        let body_name = unmodified_fn_name_from_fn_name(&body_name);
        let body_expr = if is_island {
            quote! {
                ::leptos::reactive::owner::Owner::new().with(|| {
                    ::leptos::reactive::owner::Owner::with_hydration(move || {
                        ::leptos::tachys::reactive_graph::OwnedView::new({
                            #body_name(#prop_names)
                        })
                    })
                })
            }
        } else {
            quote! {
                #body_name(#prop_names)
            }
        };

        let component = if *is_transparent {
            body_expr
        } else if cfg!(feature = "__internal_erase_components") {
            quote! {
                ::leptos::prelude::IntoMaybeErased::into_maybe_erased(
                    ::leptos::reactive::graph::untrack_with_diagnostics(
                        move || {
                            #tracing_guard_expr
                            #tracing_props_expr
                            #body_expr
                        }
                    )
                )
            }
        } else {
            quote! {
                ::leptos::reactive::graph::untrack_with_diagnostics(
                    move || {
                        #tracing_guard_expr
                        #tracing_props_expr
                        #body_expr
                    }
                )
            }
        };

        // add island wrapper if island
        let component = if is_island {
            let hydrate_fn_name = hydrate_fn_name.as_ref().unwrap();
            quote! {
                ::leptos::tachys::html::islands::Island::new(
                    stringify!(#hydrate_fn_name),
                    #component
                )
                #island_serialized_props
            }
        } else {
            component
        };

        let props_arg = if no_props {
            quote! {}
        } else {
            quote! {
                props: #props_name #generics
            }
        };

        let destructure_props = if no_props {
            quote! {}
        } else {
            let wrapped_children = if is_island_with_children {
                quote! {
                    use leptos::tachys::view::any_view::IntoAny;
                    let children = Box::new(|| {
                        let sc = ::leptos::reactive::owner::Owner::current_shared_context().unwrap();
                        let prev = sc.get_is_hydrating();
                        let owner = ::leptos::reactive::owner::Owner::new();
                        let value = owner.clone().with(|| {
                            ::leptos::reactive::owner::Owner::with_no_hydration(move || {
                                ::leptos::tachys::reactive_graph::OwnedView::new({
                                    ::leptos::tachys::html::islands::IslandChildren::new_with_on_hydrate(
                                        children(),
                                        {
                                            let owner = owner.clone();
                                            move || {
                                                owner.set()
                                            }
                                        }

                                    )
                                }).into_any()
                            })
                        });
                        sc.set_is_hydrating(prev);
                        value
                    });
                }
            } else {
                quote! {}
            };
            quote! {
                #island_serialize_props
                let #props_name {
                    #prop_names
                    ..
                } = props;
                #wrapped_children
            }
        };

        let body = quote! {
            #destructure_props
            #tracing_span_expr
            #component
        };

        let binding = if is_island {
            let island_props = if is_island_with_children
                || is_island_with_other_props
            {
                let (destructure, prop_builders, optional_props) =
                    if is_island_with_other_props {
                        let prop_names = props
                            .iter()
                            .filter_map(|prop| {
                                if prop.name.ident == "children" {
                                    None
                                } else {
                                    let name = &prop.name.ident;
                                    Some(quote! { #name, })
                                }
                            })
                            .collect::<TokenStream>();
                        let destructure = quote! {
                            let #props_serialized_name {
                                #prop_names
                            } = props;
                        };
                        let prop_builders = props
                            .iter()
                            .filter_map(|prop| {
                                if prop.name.ident == "children"
                                    || prop.prop_opts.optional
                                {
                                    None
                                } else {
                                    let name = &prop.name.ident;
                                    Some(quote! {
                                        .#name(#name)
                                    })
                                }
                            })
                            .collect::<TokenStream>();
                        let optional_props = props
                            .iter()
                            .filter_map(|prop| {
                                if prop.name.ident == "children"
                                    || !prop.prop_opts.optional
                                {
                                    None
                                } else {
                                    let name = &prop.name.ident;
                                    Some(quote! {
                                        if let Some(#name) = #name {
                                            props.#name = Some(#name)
                                        }
                                    })
                                }
                            })
                            .collect::<TokenStream>();
                        (destructure, prop_builders, optional_props)
                    } else {
                        (quote! {}, quote! {}, quote! {})
                    };
                let children = if is_island_with_children {
                    quote! {
                        .children({
                            let owner = leptos::reactive::owner::Owner::current();
                            Box::new(move || {
                            use leptos::tachys::view::any_view::IntoAny;
                            ::leptos::tachys::html::islands::IslandChildren::new_with_on_hydrate(
                                (),
                                {
                                    let owner = owner.clone();
                                    move || {
                                        if let Some(owner) = &owner {
                                            owner.set()
                                        }
                                    }
                                }
                            ).into_any()})})
                    }
                } else {
                    quote! {}
                };

                quote! {{
                    #destructure
                    let mut props = #props_name::builder()
                        #prop_builders
                        #children
                        .build();

                    #optional_props

                    props
                }}
            } else {
                quote! {}
            };
            let deserialize_island_props = if is_island_with_other_props {
                quote! {
                    let props = el.dataset().get(::leptos::wasm_bindgen::intern("props"))
                        .and_then(|data| ::leptos::serde_json::from_str::<#props_serialized_name>(&data).ok())
                        .expect("could not deserialize props");
                }
            } else {
                quote! {}
            };

            let hydrate_fn_name = hydrate_fn_name.as_ref().unwrap();

            let hydrate_fn_inner = quote! {
                #deserialize_island_props
                let island = #name(#island_props);
                let state = island.hydrate_from_position::<true>(&el, ::leptos::tachys::view::Position::Current);
                // TODO better cleanup
                std::mem::forget(state);
            };
            if *is_lazy {
                let outer_name =
                    Ident::new(&format!("{name}_loader"), name.span());

                quote! {
                    #[::leptos::prelude::lazy]
                    #[allow(non_snake_case)]
                    fn #outer_name (el: ::leptos::web_sys::HtmlElement) {
                        #hydrate_fn_inner
                    }

                    #[::leptos::wasm_bindgen::prelude::wasm_bindgen(
                        wasm_bindgen = ::leptos::wasm_bindgen,
                        wasm_bindgen_futures = ::leptos::__reexports::wasm_bindgen_futures
                    )]
                    #[allow(non_snake_case)]
                    pub async fn #hydrate_fn_name(el: ::leptos::web_sys::HtmlElement) {
                        #outer_name(el).await
                    }
                }
            } else {
                quote! {
                    #[::leptos::wasm_bindgen::prelude::wasm_bindgen(wasm_bindgen = ::leptos::wasm_bindgen)]
                    #[allow(non_snake_case)]
                    pub fn #hydrate_fn_name(el: ::leptos::web_sys::HtmlElement) {
                        #hydrate_fn_inner
                    }
                }
            }
        } else {
            quote! {}
        };

        let props_derive_serialize = if is_island_with_other_props {
            quote! { , ::leptos::serde::Serialize }
        } else {
            quote! {}
        };

        let phantom_field = generate_phantom_field(
            &phantom_type_params,
            is_island_with_other_props,
        );

        let prop_pairs: Vec<(&Ident, &Type)> =
            props.iter().map(|p| (&p.name.ident, &p.ty)).collect();
        let ModuleCheckTokens {
            module_check_traits,
            check_trait_impls,
        } = generate_module_checks(
            original_generics,
            name,
            name,
            "component",
            &prop_pairs,
            &field_types,
        );

        let required_fields: Vec<(&Ident, bool)> = props
            .iter()
            .map(|p| {
                let required = !p.prop_opts.is_optional();
                (&p.name.ident, required)
            })
            .collect();

        let ModuleRequiredCheckTokens { marker_traits } =
            generate_module_required_check(name, "component", &required_fields);

        let ModulePresenceTokens {
            module_items: module_presence_items,
            check_presence_impl,
        } = generate_module_presence_check(name, name, &required_fields);

        let module_builder = generate_module_builder(
            no_props,
            &behavioral_bounds_stripped_generics,
            &props_name,
        );

        let output = quote! {
            #[doc = #builder_name_doc]
            #[doc = ""]
            #docs_and_prop_docs
            #[derive(::leptos::typed_builder_macro::TypedBuilder #props_derive_serialize)]
            //#[builder(doc)]
            #[builder(crate_module_path=::leptos::typed_builder)]
            #[allow(non_snake_case)]
            #vis struct #props_name #struct_impl_generics #struct_where_clause {
                #prop_builder_fields
                #phantom_field
            }

            #props_serializer

            #[allow(missing_docs)]
            #binding

            impl #struct_impl_generics ::leptos::component::Props for #props_name #generics #struct_where_clause {
                type Builder = #props_builder_name #generics;

                fn builder() -> Self::Builder {
                    #props_name::builder()
                }
            }

            #unknown_attrs
            #docs_and_prop_docs
            #[allow(non_snake_case, clippy::too_many_arguments)]
            #[allow(clippy::needless_lifetimes)]
            #tracing_instrument_attr
            #vis fn #name #impl_generics (
                #props_arg
            ) #ret
            #where_clause
            {
                #body
            }

            #marker_traits

            // Companion module — coexists with `fn #name` because
            // modules live in the type namespace and functions in
            // the value namespace.  `use path::Component as Alias`
            // renames both, so the module follows renamed imports.
            #[doc(hidden)]
            #[allow(non_snake_case)]
            #vis mod #name {
                #[allow(unused_imports)]
                use super::*;
                #module_builder
                #(#module_check_traits)*
                #module_presence_items
            }

            #(#check_trait_impls)*
            #check_presence_impl
        };

        tokens.append_all(output)
    }
}

impl Model {
    #[allow(clippy::wrong_self_convention)]
    pub fn is_transparent(mut self, is_transparent: bool) -> Self {
        self.is_transparent = is_transparent;

        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_lazy(mut self, is_lazy: bool) -> Self {
        self.is_lazy = is_lazy;

        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn with_island(mut self, island: Option<String>) -> Self {
        self.island = island;

        self
    }
}

/// A model that is more lenient in case of a syntax error in the function body,
/// but does not actually implement the behavior of the real model. This is
/// used to improve IDEs and rust-analyzer's auto-completion behavior in case
/// of a syntax error.
pub struct DummyModel {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub body: TokenStream,
}

impl Parse for DummyModel {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        // Drop unknown attributes like #[deprecated]
        drain_filter(&mut attrs, |attr| {
            !is_lint_attr(attr) && !attr.path().is_ident("doc")
        });

        let vis: Visibility = input.parse()?;
        let mut sig: Signature = input.parse()?;
        maybe_modify_return_type(&mut sig.output);

        // The body is left untouched, so it will not cause an error
        // even if the syntax is invalid.
        let body: TokenStream = input.parse()?;

        Ok(Self {
            attrs,
            vis,
            sig,
            body,
        })
    }
}

impl ToTokens for DummyModel {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attrs,
            vis,
            sig,
            body,
        } = self;

        // Strip attributes like documentation comments and #[prop]
        // from the signature, so as to not confuse the user with incorrect
        // error messages.
        let sig = {
            let mut sig = sig.clone();
            sig.inputs.iter_mut().for_each(|arg| {
                if let FnArg::Typed(ty) = arg {
                    ty.attrs.retain(|attr| match &attr.meta {
                        Meta::List(list) => list
                            .path
                            .segments
                            .first()
                            .map(|n| n.ident != "prop")
                            .unwrap_or(true),
                        Meta::NameValue(name_value) => name_value
                            .path
                            .segments
                            .first()
                            .map(|n| n.ident != "doc")
                            .unwrap_or(true),
                        _ => true,
                    });
                }
            });
            sig
        };

        let output = quote! {
            #(#attrs)*
            #vis #sig #body
        };

        tokens.append_all(output)
    }
}

struct Prop {
    docs: Docs,
    prop_opts: PropOpt,
    name: PatIdent,
    ty: Type,
}

impl Prop {
    fn new(arg: FnArg) -> Self {
        let typed = if let FnArg::Typed(ty) = arg {
            ty
        } else {
            abort!(arg, "receiver not allowed in `fn`");
        };

        let prop_opts =
            PropOpt::from_attributes(&typed.attrs).unwrap_or_else(|e| {
                // TODO: replace with `.unwrap_or_abort()` once https://gitlab.com/CreepySkeleton/proc-macro-error/-/issues/17 is fixed
                abort!(e.span(), e.to_string());
            });

        let name = match *typed.pat {
            Pat::Ident(i) => {
                if let Some(name) = &prop_opts.name {
                    PatIdent {
                        attrs: vec![],
                        by_ref: None,
                        mutability: None,
                        ident: Ident::new(name, i.span()),
                        subpat: None,
                    }
                } else {
                    i
                }
            }
            Pat::Struct(_) | Pat::Tuple(_) | Pat::TupleStruct(_) => {
                if let Some(name) = &prop_opts.name {
                    PatIdent {
                        attrs: vec![],
                        by_ref: None,
                        mutability: None,
                        ident: Ident::new(name, typed.pat.span()),
                        subpat: None,
                    }
                } else {
                    abort!(
                        typed.pat,
                        "destructured props must be given a name e.g. \
                         #[prop(name = \"data\")]"
                    );
                }
            }
            _ => {
                abort!(
                    typed.pat,
                    "only `prop: bool` style types are allowed within the \
                     `#[component]` macro"
                );
            }
        };

        Self {
            docs: Docs::new(&typed.attrs),
            prop_opts,
            name,
            ty: *typed.ty,
        }
    }
}

#[derive(Clone)]
pub struct Docs(Vec<(String, Span)>);

impl ToTokens for Docs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = self
            .0
            .iter()
            .map(|(doc, span)| quote_spanned!(*span=> #[doc = #doc]))
            .collect::<TokenStream>();

        tokens.append_all(s);
    }
}

impl Docs {
    pub fn new(attrs: &[Attribute]) -> Self {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        enum ViewCodeFenceState {
            Outside,
            Rust,
            Rsx,
        }
        let mut quotes = "```".to_string();
        let mut quote_ws = "".to_string();
        let mut view_code_fence_state = ViewCodeFenceState::Outside;
        // todo fix docs stuff
        const RSX_START: &str = "# ::leptos::view! {";
        const RSX_END: &str = "# };";

        // Separated out of chain to allow rustfmt to work
        let map = |(doc, span): (String, Span)| {
            doc.split('\n')
                .map(str::trim_end)
                .flat_map(|doc| {
                    let trimmed_doc = doc.trim_start();
                    let leading_ws = &doc[..doc.len() - trimmed_doc.len()];
                    let trimmed_doc = trimmed_doc.trim_end();
                    match view_code_fence_state {
                        ViewCodeFenceState::Outside
                            if trimmed_doc.starts_with("```")
                                && trimmed_doc
                                    .trim_start_matches('`')
                                    .starts_with("view") =>
                        {
                            view_code_fence_state = ViewCodeFenceState::Rust;
                            let view = trimmed_doc.find('v').unwrap();
                            trimmed_doc[..view].clone_into(&mut quotes);
                            leading_ws.clone_into(&mut quote_ws);
                            let rust_options = &trimmed_doc
                                [view + "view".len()..]
                                .trim_start();
                            vec![
                                format!("{leading_ws}{quotes}{rust_options}"),
                                format!("{leading_ws}"),
                            ]
                        }
                        ViewCodeFenceState::Rust if trimmed_doc == quotes => {
                            view_code_fence_state = ViewCodeFenceState::Outside;
                            vec![format!("{leading_ws}"), doc.to_owned()]
                        }
                        ViewCodeFenceState::Rust
                            if trimmed_doc.starts_with('<') =>
                        {
                            view_code_fence_state = ViewCodeFenceState::Rsx;
                            vec![
                                format!("{leading_ws}{RSX_START}"),
                                doc.to_owned(),
                            ]
                        }
                        ViewCodeFenceState::Rsx if trimmed_doc == quotes => {
                            view_code_fence_state = ViewCodeFenceState::Outside;
                            vec![
                                format!("{leading_ws}{RSX_END}"),
                                doc.to_owned(),
                            ]
                        }
                        _ => vec![doc.to_string()],
                    }
                })
                .map(|l| (l, span))
                .collect_vec()
        };

        let mut attrs = attrs
            .iter()
            .filter_map(|attr| {
                let Meta::NameValue(attr) = &attr.meta else {
                    return None;
                };
                if !attr.path.is_ident("doc") {
                    return None;
                }

                let Some(val) = value_to_string(&attr.value) else {
                    abort!(
                        attr,
                        "expected string literal in value of doc comment"
                    );
                };

                Some((val, attr.path.span()))
            })
            .flat_map(map)
            .collect_vec();

        if view_code_fence_state != ViewCodeFenceState::Outside {
            if view_code_fence_state == ViewCodeFenceState::Rust {
                attrs.push((quote_ws.clone(), Span::call_site()))
            } else {
                attrs.push((format!("{quote_ws}{RSX_END}"), Span::call_site()))
            }
            attrs.push((format!("{quote_ws}{quotes}"), Span::call_site()))
        }

        Self(attrs)
    }

    pub fn padded(&self) -> TokenStream {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, (doc, span))| {
                let doc = if idx == 0 {
                    format!("    - {doc}")
                } else {
                    format!("      {doc}")
                };

                let doc = LitStr::new(&doc, *span);

                quote! { #[doc = #doc] }
            })
            .collect()
    }

    pub fn typed_builder(&self) -> String {
        let doc_str = self.0.iter().map(|s| s.0.as_str()).join("\n");

        if doc_str.chars().filter(|c| *c != '\n').count() != 0 {
            format!("\n\n{doc_str}")
        } else {
            String::new()
        }
    }
}

fn is_lint_attr(attr: &Attribute) -> bool {
    let path = &attr.path();
    path.is_ident("allow")
        || path.is_ident("warn")
        || path.is_ident("expect")
        || path.is_ident("deny")
        || path.is_ident("forbid")
}

pub struct UnknownAttrs(Vec<(TokenStream, Span)>);

impl UnknownAttrs {
    pub fn new(attrs: &[Attribute]) -> Self {
        let attrs = attrs
            .iter()
            .filter_map(|attr| {
                if attr.path().is_ident("doc") {
                    if let Meta::NameValue(_) = &attr.meta {
                        return None;
                    }
                }

                if is_lint_attr(attr) {
                    return None;
                }

                Some((attr.into_token_stream(), attr.span()))
            })
            .collect_vec();
        Self(attrs)
    }
}

impl ToTokens for UnknownAttrs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = self
            .0
            .iter()
            .map(|(attr, span)| quote_spanned!(*span=> #attr))
            .collect::<TokenStream>();
        tokens.append_all(s);
    }
}

#[derive(Clone, Debug, FromAttr)]
#[attribute(ident = prop)]
struct PropOpt {
    #[attribute(conflicts = [optional_no_strip, strip_option])]
    optional: bool,
    #[attribute(conflicts = [optional, strip_option])]
    optional_no_strip: bool,
    #[attribute(conflicts = [optional, optional_no_strip])]
    strip_option: bool,
    #[attribute(example = "5 * 10")]
    default: Option<syn::Expr>,
    into: bool,
    attrs: bool,
    name: Option<String>,
}

impl PropOpt {
    fn is_optional(&self) -> bool {
        self.optional
            || self.optional_no_strip
            || self.attrs
            || self.default.is_some()
    }
}

fn prop_builder_fields(
    vis: &Visibility,
    props: &[Prop],
    is_island_with_other_props: bool,
) -> TokenStream {
    props
        .iter()
        .map(|prop| {
            let Prop {
                docs,
                name,
                prop_opts,
                ty,
            } = prop;

            let builder_attrs = typed_builder_opts(
                prop_opts.is_optional(),
                &prop_opts.default,
                prop_opts.strip_option,
                prop_opts.optional,
                prop_opts.into,
                ty,
            );

            let doc_input = PropDocInput {
                docs,
                name: &name.ident,
                ty,
                is_optional: prop_opts.is_optional(),
                optional: prop_opts.optional,
                strip_option: prop_opts.strip_option,
                into: prop_opts.into,
            };
            let builder_docs = prop_to_doc(&doc_input, PropDocStyle::Inline);

            // Children won't need documentation in many cases
            let allow_missing_docs = if name.ident == "children" {
                quote!(#[allow(missing_docs)])
            } else {
                quote!()
            };
            let skip_children_serde =
                if is_island_with_other_props && name.ident == "children" {
                    quote!(#[serde(skip)])
                } else {
                    quote!()
                };

            let PatIdent { ident, by_ref, .. } = &name;

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                #allow_missing_docs
                #skip_children_serde
                #vis #by_ref #ident: #ty,
            }
        })
        .collect()
}

fn prop_serializer_fields(vis: &Visibility, props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter_map(|prop| {
            if prop.name.ident == "children" {
                None
            } else {
                let Prop {
                    docs,
                    name,
                    prop_opts,
                    ty,
                } = prop;

                let builder_attrs = typed_builder_opts(
                    prop_opts.is_optional(),
                    &prop_opts.default,
                    prop_opts.strip_option,
                    prop_opts.optional,
                    prop_opts.into,
                    ty,
                );
                let serde_attrs = builder_attrs.to_serde_tokens();

                let PatIdent { ident, by_ref, .. } = &name;

                Some(quote! {
                    #docs
                    #serde_attrs
                    #vis #by_ref #ident: #ty,
                })
            }
        })
        .collect()
}

fn prop_names(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .map(|Prop { name, .. }| {
            // fields like mutability are removed because unneeded
            // in the contexts in which this is used
            let ident = &name.ident;
            quote! { #ident, }
        })
        .collect()
}

pub fn unmodified_fn_name_from_fn_name(ident: &Ident) -> Ident {
    Ident::new(
        &format!("__component_{}", ident.to_string().to_case(Snake)),
        ident.span(),
    )
}

/// Converts all `impl Trait`s in a function signature to use generic params instead.
fn convert_impl_trait_to_generic(sig: &mut Signature) {
    fn new_generic_ident(i: usize, span: Span) -> Ident {
        Ident::new(&format!("__ImplTrait{i}"), span)
    }

    // First: visit all `impl Trait`s and replace them with new generic params.
    #[derive(Default)]
    struct RemoveImplTrait(Vec<TypeImplTrait>);
    impl VisitMut for RemoveImplTrait {
        fn visit_type_mut(&mut self, ty: &mut Type) {
            syn::visit_mut::visit_type_mut(self, ty);
            if matches!(ty, Type::ImplTrait(_)) {
                let ident = new_generic_ident(self.0.len(), ty.span());
                let generic_type = Type::Path(TypePath {
                    qself: None,
                    path: Path::from(ident),
                });
                let Type::ImplTrait(impl_trait) =
                    std::mem::replace(ty, generic_type)
                else {
                    unreachable!();
                };
                self.0.push(impl_trait);
            }
        }

        // Early exits.
        fn visit_attribute_mut(&mut self, _: &mut Attribute) {}
        fn visit_pat_mut(&mut self, _: &mut Pat) {}
    }
    let mut visitor = RemoveImplTrait::default();
    for fn_arg in sig.inputs.iter_mut() {
        visitor.visit_fn_arg_mut(fn_arg);
    }
    let RemoveImplTrait(impl_traits) = visitor;

    // Second: Add the new generic params into the signature.
    for (i, impl_trait) in impl_traits.into_iter().enumerate() {
        let span = impl_trait.span();
        let ident = new_generic_ident(i, span);
        // We can simply append to the end (only lifetime params must be first).
        // Note currently default generics are not allowed in `fn`, so not a concern.
        sig.generics.params.push(GenericParam::Type(TypeParam {
            attrs: vec![],
            ident,
            colon_token: Some(Colon { spans: [span] }),
            bounds: impl_trait.bounds,
            eq_token: None,
            default: None,
        }));
    }
}
