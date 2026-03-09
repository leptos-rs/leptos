use crate::util::{
    children::is_children_prop,
    documentation::{
        generate_prop_documentation, prop_to_doc, Docs, PropDocumentationStyle,
    },
    generate_companion_internals, type_analysis,
    typed_builder_opts::TypedBuilderOpts,
    CompanionConfig, CompanionModuleBody, PropLike,
};
use attribute_derive::FromAttr;
use convert_case::{
    Case::{Pascal, Snake},
    Casing,
};
use convert_case_extras::is_case;
use itertools::Itertools;
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
    props: Vec<ComponentProp>,
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
            .map(ComponentProp::new)
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
/// by replacing the return type with AnyNestedRoute, which is what it'll be,
/// but is required as the return type for compiler inference.
fn maybe_modify_return_type(ret: &mut ReturnType) {
    #[cfg(feature = "__internal_erase_components")]
    {
        if let ReturnType::Type(_, ty) = ret {
            if let Type::ImplTrait(TypeImplTrait { bounds, .. }) = ty.as_ref() {
                // If one of the bounds is MatchNestedRoutes, we need to replace
                // the return type with AnyNestedRoute:
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

        check_component_name_against_prelude(name);

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

        // Keep a reference to the full original generics before `body` is
        // shadowed later by a quote! block.
        let original_generics = &body.sig.generics;

        let (impl_generics, generics, where_clause) =
            original_generics.split_for_impl();

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
        let struct_generics = type_analysis::strip_non_structural_bounds(
            original_generics,
            &field_types,
        );
        let (struct_impl_generics, _, struct_where_clause) =
            struct_generics.split_for_impl();

        let phantom_type_params = type_analysis::find_unused_type_params(
            original_generics,
            &field_types,
        );

        let props_name = format_ident!("{name}Props");
        let props_builder_name = format_ident!("{name}PropsBuilder");
        let props_serialized_name = format_ident!("{name}PropsSerialized");
        #[cfg(feature = "tracing")]
        let trace_name = format!("<{name} />");

        let is_island_with_children = is_island
            && props
                .iter()
                .any(|prop| is_children_prop(&prop.name.ident, &prop.ty));
        let is_island_with_other_props = is_island
            && ((is_island_with_children && props.len() > 1)
                || (!is_island_with_children && !props.is_empty()));

        let prop_builder_fields =
            prop_builder_fields(props, is_island_with_other_props);
        let props_serializer = if is_island_with_other_props {
            let fields = prop_serializer_fields(props);
            quote! {
                #[derive(::leptos::serde::Deserialize)]
                pub struct #props_serialized_name {
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

        let component_fn_prop_docs = generate_prop_documentation(props);
        let docs_and_prop_docs = if component_fn_prop_docs.is_empty() {
            // Avoid generating an empty doc line in case the component has no
            // doc and no props.
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
                 * The problem is that cargo now warns about an expected
                 * "tracing" cfg if you don't have a
                 * "tracing" feature in your actual crate
                 *
                 * However, until https://github.com/tokio-rs/tracing/pull/1819 is merged
                 * (?), you can't provide an alternate path for `tracing`
                 * (for example, ::leptos::tracing), which
                 * means that if you're going to use the macro
                 * you *must* have `tracing` in your Cargo.toml.
                 *
                 * Including the feature-check here causes cargo warnings on
                 * previously-working projects.
                 *
                 * Removing the feature-check here breaks any project that
                 * uses leptos with the tracing feature
                 * turned on, but without a tracing dependency in its
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

        let body_name = component_inner_fn_name(&body_name);
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
                                if is_children_prop(&prop.name.ident, &prop.ty)
                                {
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
                                if is_children_prop(&prop.name.ident, &prop.ty)
                                    || prop.options.optional
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
                                if is_children_prop(&prop.name.ident, &prop.ty)
                                    || !prop.options.optional
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

        let phantom_field = type_analysis::generate_phantom_field(
            &phantom_type_params,
            is_island_with_other_props,
        );

        // Prefixed to avoid type-namespace collisions with user defined types
        // from the same scope. Allows users to write
        // ```
        // struct Foo;
        // #[component]
        // fn Foo(_foo: Foo) -> impl IntoView { () }
        // ```
        // without resulting in both module and struct named equally.
        // Using the `::leptos::component::component_helper` inference bridge
        // in view! macros stil allows for renamed imports of component fns,
        // as no direct (named) access to this companion module is required.
        let companion_name = format_ident!("__{}", name);

        let CompanionModuleBody {
            module_items,
            trait_impls,
            helper_constructor_arg,
        } = generate_companion_internals(&CompanionConfig {
            original_generics,
            stripped_generics: &struct_generics,
            module_name: &companion_name,
            display_name: name,
            kind: "component",
            props_name: &props_name,
            props: &props,
        });

        let props_serialized_reexport = if is_island_with_other_props {
            quote! { #vis use #companion_name::#props_serialized_name; }
        } else {
            quote! {}
        };

        let props_arg = if no_props {
            quote! {}
        } else {
            quote! {
                props: #props_name #generics
            }
        };

        let output = quote! {
            #[allow(missing_docs)]
            #binding

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

            #[doc(hidden)]
            #[allow(non_snake_case)]
            #vis mod #companion_name {
                #[allow(unused_imports)]
                use super::*;

                #[doc = #builder_name_doc]
                #[doc = ""]
                #docs_and_prop_docs
                #[derive(::leptos::typed_builder_macro::TypedBuilder #props_derive_serialize)]
                //#[builder(doc)]
                #[builder(crate_module_path=::leptos::typed_builder)]
                #[allow(non_snake_case)]
                pub struct #props_name #struct_impl_generics #struct_where_clause {
                    #prop_builder_fields
                    #phantom_field
                }

                #props_serializer

                impl #struct_impl_generics ::leptos::component::Props for #props_name #generics #struct_where_clause {
                    type Builder = #props_builder_name #generics;
                    type Helper = Helper #generics;

                    fn builder() -> Self::Builder {
                        #props_name::builder()
                    }

                    fn helper() -> Self::Helper {
                        __helper()
                    }
                }

                #module_items

                #[doc(hidden)]
                pub fn __helper #struct_impl_generics ()
                    -> Helper #generics
                    #struct_where_clause
                {
                    Helper(#helper_constructor_arg)
                }
            }

            #vis use #companion_name::#props_name;
            #vis use #companion_name::#props_builder_name;
            #props_serialized_reexport

            #trait_impls
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

struct ComponentProp {
    docs: Docs,
    options: ComponentPropOptions,
    name: PatIdent,
    ty: Type,
}

impl PropLike for ComponentProp {
    fn name(&self) -> &Ident {
        &self.name.ident
    }
    fn ty(&self) -> &Type {
        &self.ty
    }
    fn docs(&self) -> &Docs {
        &self.docs
    }
    fn is_optional(&self) -> bool {
        self.options.is_optional()
    }
    fn optional(&self) -> bool {
        self.options.optional
    }
    fn strip_option(&self) -> bool {
        self.options.strip_option
    }
    fn into_prop(&self) -> bool {
        self.options.into
    }
    fn default(&self) -> Option<&syn::Expr> {
        self.options.default.as_ref()
    }
}

impl ComponentProp {
    fn new(arg: FnArg) -> Self {
        let typed = if let FnArg::Typed(ty) = arg {
            ty
        } else {
            abort!(arg, "receiver not allowed in `fn`");
        };

        let options = ComponentPropOptions::from_attributes(&typed.attrs)
            .unwrap_or_else(|e| {
                // TODO: replace with `.unwrap_or_abort()` once https://gitlab.com/CreepySkeleton/proc-macro-error/-/issues/17 is fixed
                abort!(e.span(), e.to_string());
            });

        let name = match *typed.pat {
            Pat::Ident(i) => {
                if let Some(name) = &options.name {
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
                if let Some(name) = &options.name {
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
            options,
            name,
            ty: *typed.ty,
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

/// A list of names which cannot be used by Leptos users to name their own
/// components.
const FORBIDDEN_TYPE_NAMES: &[&str] = &[
    "Component",
    "Props",
    "Children",
    "View",
    "Fragment",
    "Signal",
    "Memo",
    "Effect",
    "Action",
    "Resource",
    "Callback",
    "Owner",
];

/// Checks whether the component `name` is forbidden.
///
/// `#[component]` generates a companion module with the same name as the
/// component function. Because modules live in the type namespace, they can
/// conflict with structs, traits, enums, and type aliases imported through, for
/// example, `leptos::prelude::*`, or other sources.
///
/// when a user tries to name a component the same way a leptos-provided item is
/// already named, Rust would produce an unhelpful E0659 (ambiguous name) error.
///
/// We catch the most common collisions at compile-time and emit a clear error
/// instead
fn check_component_name_against_prelude(name: &Ident) {
    // Skip the check when compiling the `leptos` crate itself, which
    // defines some of the items in our list (e.g. `Component`, `View`).
    if std::env::var("CARGO_PKG_NAME").as_deref() == Ok("leptos") {
        return;
    }

    let name_str = name.to_string();
    if FORBIDDEN_TYPE_NAMES.contains(&name_str.as_str()) {
        abort!(
            name.span(),
            "component name `{}` conflicts with `leptos::prelude::{}`",
            name_str, name_str;
            help = "rename the component to avoid the conflict, e.g. `My{}` or `App{}`",
            name_str, name_str
        );
    }
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
struct ComponentPropOptions {
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

impl ComponentPropOptions {
    fn is_optional(&self) -> bool {
        self.optional
            || self.optional_no_strip
            || self.attrs
            || self.default.is_some()
    }
}

fn prop_builder_fields(
    props: &[ComponentProp],
    is_island_with_other_props: bool,
) -> TokenStream {
    props
        .iter()
        .map(|prop| {
            let builder_attrs = TypedBuilderOpts::from_prop(prop);
            let builder_docs =
                prop_to_doc(prop, PropDocumentationStyle::Inline);

            let name = &prop.name;
            let ty = prop.ty();
            let docs = prop.docs();

            // Children won't need documentation in many cases
            let allow_missing_docs = if is_children_prop(&name.ident, ty) {
                quote!(#[allow(missing_docs)])
            } else {
                quote!()
            };
            let skip_children_serde = if is_island_with_other_props
                && is_children_prop(&name.ident, ty)
            {
                quote!(#[serde(skip)])
            } else {
                quote!()
            };

            let PatIdent { ident, by_ref, .. } = name;

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                #allow_missing_docs
                #skip_children_serde
                pub #by_ref #ident: #ty,
            }
        })
        .collect()
}

fn prop_serializer_fields(props: &[ComponentProp]) -> TokenStream {
    props
        .iter()
        .filter_map(|prop| {
            if is_children_prop(&prop.name.ident, prop.ty()) {
                None
            } else {
                let builder_attrs = TypedBuilderOpts::from_prop(prop);
                let serde_attrs = builder_attrs.to_serde_tokens();

                let docs = prop.docs();
                let PatIdent { ident, by_ref, .. } = &prop.name;
                let ty = prop.ty();

                Some(quote! {
                    #docs
                    #serde_attrs
                    pub #by_ref #ident: #ty,
                })
            }
        })
        .collect()
}

fn prop_names(props: &[ComponentProp]) -> TokenStream {
    props
        .iter()
        .map(|ComponentProp { name, .. }| {
            // fields like mutability are removed because unneeded
            // in the contexts in which this is used
            let ident = &name.ident;
            quote! { #ident, }
        })
        .collect()
}

pub fn component_inner_fn_name(ident: &Ident) -> Ident {
    Ident::new(
        &format!("__component_{}", ident.to_string().to_case(Snake)),
        ident.span(),
    )
}

/// Converts all `impl Trait`s in a function signature to use generic params
/// instead.
fn convert_impl_trait_to_generic(sig: &mut Signature) {
    fn new_generic_ident(i: usize, span: Span) -> Ident {
        Ident::new(&format!("__ImplTrait{i}"), span)
    }

    // First: visit all `impl Trait`s and replace them with new generic params.
    #[derive(Default)]
    struct ReplaceImplTraitWithGeneric(Vec<TypeImplTrait>);
    impl VisitMut for ReplaceImplTraitWithGeneric {
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
    let mut visitor = ReplaceImplTraitWithGeneric::default();
    for fn_arg in sig.inputs.iter_mut() {
        visitor.visit_fn_arg_mut(fn_arg);
    }
    let ReplaceImplTraitWithGeneric(impl_traits) = visitor;

    // Second: Add the new generic params into the signature.
    for (i, impl_trait) in impl_traits.into_iter().enumerate() {
        let span = impl_trait.span();
        let ident = new_generic_ident(i, span);
        // We can simply append to the end (only lifetime params must be first).
        // Note currently default generics are not allowed in `fn`, so not a
        // concern.
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
