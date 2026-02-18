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
    parse::Parse, parse_quote, spanned::Spanned, token::Colon, visit::Visit,
    visit_mut::VisitMut, AngleBracketedGenericArguments, Attribute, FnArg,
    GenericArgument, GenericParam, Item, ItemFn, LitStr, Meta, Pat, PatIdent,
    Path, PathArguments, ReturnType, Signature, Stmt, Type, TypeImplTrait,
    TypeParam, TypePath, Visibility, WherePredicate,
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
        let struct_generics =
            strip_non_structural_bounds(&body.sig.generics, &field_types);
        let (struct_impl_generics, _, struct_where_clause) =
            struct_generics.split_for_impl();

        let phantom_type_params =
            collect_phantom_type_params(&body.sig.generics, &field_types);

        let props_name = format_ident!("{name}Props");
        let props_builder_name = format_ident!("{name}PropsBuilder");
        let companion_name = format_ident!("{name}__");
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

        let component_fn_prop_docs = generate_component_fn_prop_docs(props);
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
        let CompanionCheckTokens {
            prop_traits,
            check_methods,
        } = generate_companion_checks(
            original_generics,
            name,
            &prop_pairs,
            &field_types,
        );

        let required_fields: Vec<(&Ident, bool)> = props
            .iter()
            .map(|p| {
                let required = !p.prop_opts.optional
                    && !p.prop_opts.optional_no_strip
                    && !p.prop_opts.attrs
                    && p.prop_opts.default.is_none();
                (&p.name.ident, required)
            })
            .collect();
        let RequiredCheckTokens {
            marker_traits,
            check_required_method,
        } = generate_required_check(
            name,
            &props_builder_name,
            original_generics,
            &required_fields,
        );

        // Builder method on the companion struct.
        let builder_method = if no_props {
            quote! {
                #[doc(hidden)]
                pub fn builder()
                    -> ::leptos::component::EmptyPropsBuilder
                {
                    ::leptos::component::EmptyPropsBuilder {}
                }
            }
        } else {
            quote! {
                #[doc(hidden)]
                pub fn builder #struct_impl_generics ()
                    -> <#props_name #generics
                        as ::leptos::component::Props>::Builder
                    #struct_where_clause
                {
                    <#props_name #generics
                        as ::leptos::component::Props>::builder()
                }
            }
        };

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

            #(#prop_traits)*
            #marker_traits
            #check_required_method

            #[doc(hidden)]
            #[allow(non_snake_case)]
            #vis struct #companion_name {}

            #[doc(hidden)]
            impl #companion_name {
                #builder_method
                #(#check_methods)*
            }

            // Type alias so that `ComponentName::builder()` and renamed
            // imports (`use path::Component as Alias; Alias::builder()`)
            // resolve through the alias to the companion struct.
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis type #name = #companion_name;
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

struct TypedBuilderOpts<'a> {
    default: bool,
    default_with_value: Option<syn::Expr>,
    strip_option: bool,
    into: bool,
    ty: &'a Type,
}

impl<'a> TypedBuilderOpts<'a> {
    fn from_opts(opts: &PropOpt, ty: &'a Type) -> Self {
        Self {
            default: opts.optional || opts.optional_no_strip || opts.attrs,
            default_with_value: opts.default.clone(),
            strip_option: opts.strip_option || opts.optional && is_option(ty),
            into: opts.into,
            ty,
        }
    }
}

impl TypedBuilderOpts<'_> {
    fn to_serde_tokens(&self) -> TokenStream {
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

impl ToTokens for TypedBuilderOpts<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default_code=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        // If self.strip_option && self.into, then the strip_option will be represented as part of the transform closure.
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

            let builder_attrs = TypedBuilderOpts::from_opts(prop_opts, ty);

            let builder_docs = prop_to_doc(prop, PropDocStyle::Inline);

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

                let builder_attrs = TypedBuilderOpts::from_opts(prop_opts, ty);
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

fn generate_component_fn_prop_docs(props: &[Prop]) -> TokenStream {
    let required_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            !(prop_opts.optional
                || prop_opts.optional_no_strip
                || prop_opts.default.is_some())
        })
        .map(|p| prop_to_doc(p, PropDocStyle::List))
        .collect::<TokenStream>();

    let optional_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            prop_opts.optional
                || prop_opts.optional_no_strip
                || prop_opts.default.is_some()
        })
        .map(|p| prop_to_doc(p, PropDocStyle::List))
        .collect::<TokenStream>();

    let required_prop_docs = if !required_prop_docs.is_empty() {
        quote! {
            #[doc = " # Required Props"]
            #required_prop_docs
        }
    } else {
        quote! {}
    };

    let optional_prop_docs = if !optional_prop_docs.is_empty() {
        quote! {
            #[doc = " # Optional Props"]
            #optional_prop_docs
        }
    } else {
        quote! {}
    };

    quote! {
        #required_prop_docs
        #optional_prop_docs
    }
}

pub fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
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

pub fn unwrap_option(ty: &Type) -> Type {
    const STD_OPTION_MSG: &str =
        "make sure you're not shadowing the `std::option::Option` type that \
         is automatically imported from the standard prelude";

    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let [first] = &segments.iter().collect::<Vec<_>>()[..] {
            if first.ident == "Option" {
                if let PathArguments::AngleBracketed(
                    AngleBracketedGenericArguments { args, .. },
                ) = &first.arguments
                {
                    if let [GenericArgument::Type(ty)] =
                        &args.iter().collect::<Vec<_>>()[..]
                    {
                        return ty.clone();
                    }
                }
            }
        }
    }

    abort!(
        ty,
        "`Option` must be `std::option::Option`";
        help = STD_OPTION_MSG
    );
}

#[derive(Clone, Copy)]
enum PropDocStyle {
    List,
    Inline,
}

fn prop_to_doc(
    Prop {
        docs,
        name,
        ty,
        prop_opts,
    }: &Prop,
    style: PropDocStyle,
) -> TokenStream {
    let ty = if (prop_opts.optional || prop_opts.strip_option) && is_option(ty)
    {
        unwrap_option(ty)
    } else {
        ty.to_owned()
    };

    let type_item: syn::Item = parse_quote! {
        type SomeType = #ty;
    };

    let file = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![type_item],
    };

    let pretty_ty = prettyplease::unparse(&file);

    let pretty_ty = &pretty_ty[16..&pretty_ty.len() - 2];

    match style {
        PropDocStyle::List => {
            let arg_ty_doc = LitStr::new(
                &if !prop_opts.into {
                    format!(" - **{}**: [`{pretty_ty}`]", quote!(#name))
                } else {
                    format!(
                        " - **{}**: [`impl Into<{pretty_ty}>`]({pretty_ty})",
                        quote!(#name),
                    )
                },
                name.ident.span(),
            );

            let arg_user_docs = docs.padded();

            quote! {
                #[doc = #arg_ty_doc]
                #arg_user_docs
            }
        }
        PropDocStyle::Inline => {
            let arg_ty_doc = LitStr::new(
                &if !prop_opts.into {
                    format!(
                        "**{}**: [`{}`]{}",
                        quote!(#name),
                        pretty_ty,
                        docs.typed_builder()
                    )
                } else {
                    format!(
                        "**{}**: `impl`[`Into<{}>`]{}",
                        quote!(#name),
                        pretty_ty,
                        docs.typed_builder()
                    )
                },
                name.ident.span(),
            );

            quote! {
                #[builder(setter(doc = #arg_ty_doc))]
            }
        }
    }
}

pub fn unmodified_fn_name_from_fn_name(ident: &Ident) -> Ident {
    Ident::new(
        &format!("__component_{}", ident.to_string().to_case(Snake)),
        ident.span(),
    )
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

/// Returns true if the type is exactly the given generic param name
/// with no wrapping.
pub(crate) fn is_bare_generic_param(ty: &Type, ident: &Ident) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.is_ident(ident)
    } else {
        false
    }
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

/// Collects generic type params that are NOT used in any field type.
/// These need `PhantomData` so the struct compiles without their
/// where-clause bounds. This commonly occurs with params like `I`,
/// `T`, `N`, `K` that only appear in where-clause constraints (e.g.
/// `IF: Fn() -> I, I: IntoIterator<Item = T>`).
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

/// The generated token streams for companion check methods.
pub(crate) struct CompanionCheckTokens {
    /// Module-level items: PropBound traits (with `on_unimplemented`
    /// for single-param generic props), per-prop wrapper structs,
    /// and `__PropPass` impls for the wrappers.
    pub prop_traits: Vec<TokenStream>,
    /// Check functions for the companion struct/slot impl block.
    /// Each returns a wrapper type. Chaining `.__pass()` on the
    /// wrapper produces E0599/{error} when bounds fail.
    pub check_methods: Vec<TokenStream>,
}

/// Generates per-prop check methods, wrapper types, and associated
/// custom traits for a component or slot.
///
/// For each prop, the classification determines the output:
///
/// - **Generic, single-param bounds**: Custom supertrait with
///   `on_unimplemented` + bounded check method returning wrapper +
///   bounded `__PropPass` impl on wrapper (E0599 → `{error}`).
/// - **Everything else** (concrete, `into`, unbounded, multi-param
///   generic): Identity check method returning wrapper + blanket
///   `__PropPass` impl. For `into` props, the builder setter's
///   `IntoReactiveValue` bound produces E0277 pointing to the
///   value argument, so no pre-check bound is needed.
///
/// - `full_generics`: the full generics from the original
///   function/struct (all bounds)
/// - `component_name`: used as prefix in trait/wrapper names to
///   avoid collisions
/// - `props`: (name, type) pairs for each prop
/// - `field_types`: all field types (for structural bounds check)
pub(crate) fn generate_companion_checks(
    full_generics: &syn::Generics,
    component_name: &Ident,
    props: &[(&Ident, &Type)],
    field_types: &[&Type],
) -> CompanionCheckTokens {
    if props.is_empty() {
        return CompanionCheckTokens {
            prop_traits: vec![],
            check_methods: vec![],
        };
    }

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

    let mut prop_traits = vec![];
    let mut check_methods = vec![];

    for (prop_name, prop_ty) in props {
        let name_str = prop_name.to_string();
        let clean_name = name_str.strip_prefix("r#").unwrap_or(&name_str);
        let check_fn = format_ident!("__check_{}", clean_name);
        let wrap_name =
            format_ident!("__{}_Wrap_{}", component_name, clean_name);

        // Determine if this is a bounded single-param generic prop.
        // This takes priority over `into` (case 1 in the doc comment).
        //
        // Three conditions must hold:
        // 1. The prop type is a bare generic param from the
        //    stripped set (not structurally needed).
        // 2. That param has non-empty predicates (bounds).
        // 3. None of those predicates reference other generic
        //    params (which would make the check method unsound
        //    in isolation).
        let bounded_single_param = stripped_params
            .iter()
            .find(|ident| is_bare_generic_param(prop_ty, ident))
            .and_then(|param_ident| {
                let preds =
                    collect_predicates_for_param(full_generics, param_ident);
                if !preds.is_empty()
                    && !bounds_reference_other_params(
                        &preds,
                        param_ident,
                        &all_generic_idents,
                    )
                {
                    Some(preds)
                } else {
                    None
                }
            });

        if let Some(param_predicates) = bounded_single_param {
            // Case 1: Bounded single-param generic prop.
            let trait_name =
                format_ident!("__{}_PropBound_{}", component_name, clean_name);
            let bounds = predicates_to_bounds(&param_predicates);
            let bounds_note = bounds.to_string();
            let message = format!(
                "`{{Self}}` is not a valid type for prop `{clean_name}` on \
                 component `{component_name}`",
            );
            let note = format!("required: `{bounds_note}`");

            prop_traits.push(quote! {
                #[doc(hidden)]
                #[diagnostic::on_unimplemented(
                    message = #message,
                    note = #note
                )]
                #[allow(non_camel_case_types)]
                pub trait #trait_name: #bounds {}
                impl<__T: #bounds> #trait_name
                    for __T
                {
                }

                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                pub struct #wrap_name<__T>(pub __T);

                #[diagnostic::do_not_recommend]
                impl<__T: #trait_name>
                    ::leptos::component::__PropPass for #wrap_name<__T>
                {
                    type Output = __T;
                    fn __pass(self) -> __T {
                        self.0
                    }
                }
            });

            check_methods.push(quote! {
                #[doc(hidden)]
                #[allow(non_snake_case)]
                pub fn #check_fn<__T: #trait_name>(
                    val: __T,
                ) -> #wrap_name<__T> {
                    #wrap_name(val)
                }
            });
        } else {
            // Case 2: `into` props. These don't need a bounded
            // pre-check because the builder setter already carries
            // the `IntoReactiveValue` bound. E0277 from the setter
            // naturally points to the value argument regardless of
            // the setter name's span, producing one clean error.
            //
            // Case 3: Concrete / unbounded / multi-param generic.
            // Identity wrapper with blanket __PropPass.
            prop_traits.push(quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                pub struct #wrap_name<__T>(pub __T);

                #[diagnostic::do_not_recommend]
                impl<__T>
                    ::leptos::component::__PropPass for #wrap_name<__T>
                {
                    type Output = __T;
                    fn __pass(self) -> __T {
                        self.0
                    }
                }
            });

            check_methods.push(quote! {
                #[doc(hidden)]
                #[allow(non_snake_case)]
                pub fn #check_fn<__T>(
                    val: __T,
                ) -> #wrap_name<__T> {
                    #wrap_name(val)
                }
            });
        }
    }

    CompanionCheckTokens {
        prop_traits,
        check_methods,
    }
}

/// Checks if any of the bounds in the given predicates reference
/// generic params other than `self_ident`.
fn bounds_reference_other_params(
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
fn predicates_to_bounds(predicates: &[WherePredicate]) -> TokenStream {
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

/// The split output of `generate_required_check`.
pub(crate) struct RequiredCheckTokens {
    /// Marker trait definitions (with `on_unimplemented`) to be
    /// placed at module scope.
    pub marker_traits: TokenStream,
    /// The `__check_missing` associated function to be placed
    /// inside the companion struct (or slot struct) impl block.
    pub check_required_method: TokenStream,
}

/// Generates marker traits and a `__check_missing` associated
/// function. When any required field is missing (its type-state
/// slot is `()`), the marker trait bound fails and the compiler
/// emits E0277 with a custom `on_unimplemented` message.
///
/// The impl block carries all bounds from `generics` (structural +
/// behavioral). This means it also serves as secondary error
/// suppression: when any behavioral bound fails, the method isn't
/// found (E0599), making the expression `{error}` which absorbs
/// downstream errors from `component_view`.
///
/// - `component_name`: used as prefix in trait names to avoid
///   collisions
/// - `builder_name`: the builder struct name (e.g.
///   `InnerPropsBuilder`)
/// - `generics`: the full generics (all bounds — structural and
///   behavioral)
/// - `fields`: `(name, is_required)` for each non-skipped field in
///   declaration order
pub(crate) fn generate_required_check(
    component_name: &Ident,
    builder_name: &Ident,
    generics: &syn::Generics,
    fields: &[(&Ident, bool)],
) -> RequiredCheckTokens {
    if fields.is_empty() {
        return RequiredCheckTokens {
            marker_traits: quote! {},
            check_required_method: quote! {},
        };
    }

    let (_, _, where_clause) = generics.split_for_impl();

    // Collect generic params for the impl header (with bounds).
    let generic_params: Vec<&GenericParam> = generics.params.iter().collect();

    // Collect generic args for type position (idents / lifetimes
    // only, no bounds).
    let type_args: Vec<TokenStream> = generics
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
        .collect();

    let mut marker_traits = vec![];
    // `type_state_params` carries bounded params (e.g. `__F0: __required_Foo_bar`)
    // for the impl header, while `type_state_idents` holds bare idents (e.g.
    // `__F0`) for the type position in `Builder<(F0, F1, ...)>`. The loop below
    // builds both in tandem: required fields get a marker-trait bound on the
    // param, optional fields get an unbounded param.
    let mut type_state_params = vec![];
    let mut type_state_idents = vec![];

    for (i, (name, required)) in fields.iter().enumerate() {
        let param = format_ident!("__F{}", i);
        type_state_idents.push(param.clone());

        if *required {
            // Strip r# prefix for the trait name.
            let name_str = name.to_string();
            let clean_name = name_str.strip_prefix("r#").unwrap_or(&name_str);
            let trait_name = Ident::new(
                &format!("__required_{component_name}_{clean_name}"),
                Span::call_site(),
            );

            let message = format!(
                "missing required prop `{clean_name}` on component \
                 `{component_name}`"
            );

            marker_traits.push(quote! {
                #[doc(hidden)]
                #[diagnostic::on_unimplemented(
                    message = #message
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

    // Generate a standalone impl block on the PropsBuilder. All
    // bounds (structural + behavioral + required-prop marker
    // traits) go on the IMPL block. When a required prop is
    // missing or a behavioral bound fails, E0599 fires (method
    // not found) → expression is `{error}` which absorbs
    // downstream errors.
    let impl_params = if generic_params.is_empty() {
        quote! { #(#type_state_params),* }
    } else {
        quote! { #(#generic_params,)* #(#type_state_params),* }
    };

    let check_required_method = quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<#impl_params> #builder_name<#builder_type_args>
        #where_clause
        {
            pub fn __check_missing(self) -> Self
            { self }
        }
    };

    RequiredCheckTokens {
        marker_traits: quote! { #(#marker_traits)* },
        check_required_method,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

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
        // Split at "where" to insert the param list between
        // the angle brackets and the where clause.
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
        // F should have no inline bounds
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty());
        } else {
            panic!("expected type param");
        }
        // Where clause should be removed
        assert!(stripped.where_clause.is_none());
    }

    #[test]
    fn strip_keeps_structural() {
        let generics = parse_generics("<F: Clone> where F: Clone");
        let ty = parse_ty("Vec<F>");
        let stripped = strip_non_structural_bounds(&generics, &[&ty]);
        // F should keep its bounds
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
        // F should be stripped
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty(), "F bounds should be stripped");
        }
        // S should keep its bounds
        if let GenericParam::Type(tp) = &stripped.params[1] {
            assert!(!tp.bounds.is_empty(), "S bounds should be kept");
        }
        // Where clause should only contain S predicates
        let wc = stripped.where_clause.as_ref().unwrap();
        assert_eq!(wc.predicates.len(), 1);
        let pred_str = wc.predicates[0].to_token_stream().to_string();
        assert!(
            pred_str.contains("S"),
            "kept predicate should be for S, got: {pred_str}"
        );
    }
}
