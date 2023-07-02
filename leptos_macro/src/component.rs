use attribute_derive::Attribute as AttributeDerive;
use convert_case::{
    Case::{Pascal, Snake},
    Casing,
};
use itertools::Itertools;
use leptos_hot_reload::parsing::value_to_string;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, spanned::Spanned,
    AngleBracketedGenericArguments, Attribute, FnArg, GenericArgument, Item,
    ItemFn, LitStr, Meta, Pat, PatIdent, Path, PathArguments, ReturnType, Stmt,
    Type, TypePath, Visibility,
};
pub struct Model {
    is_transparent: bool,
    docs: Docs,
    vis: Visibility,
    name: Ident,
    scope_name: PatIdent,
    props: Vec<Prop>,
    body: ItemFn,
    ret: ReturnType,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut item = ItemFn::parse(input)?;

        let docs = Docs::new(&item.attrs);

        let props = item
            .sig
            .inputs
            .clone()
            .into_iter()
            .map(Prop::new)
            .collect::<Vec<_>>();

        let scope_name = if props.is_empty() {
            abort!(
                item.sig,
                "this method requires a `Scope` parameter";
                help = "try `fn {}(cx: Scope, /* ... */)`", item.sig.ident
            );
        } else if !is_valid_scope_type(&props[0].ty) {
            abort!(
                item.sig.inputs,
                "this method requires a `Scope` parameter";
                help = "try `fn {}(cx: Scope, /* ... */ */)`", item.sig.ident
            );
        } else {
            props[0].name.clone()
        };

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

        // Make sure return type is correct
        if !is_valid_into_view_return_type(&item.sig.output) {
            abort!(
                item.sig,
                "return type is incorrect";
                help = "return signature must be `-> impl IntoView`"
            );
        }

        Ok(Self {
            is_transparent: false,
            docs,
            vis: item.vis.clone(),
            name: convert_from_snake_case(&item.sig.ident),
            scope_name,
            props,
            ret: item.sig.output.clone(),
            body: item,
        })
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
    if !name_str.is_case(Snake) {
        name.clone()
    } else {
        Ident::new(&name_str.to_case(Pascal), name.span())
    }
}

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            is_transparent,
            docs,
            vis,
            name,
            scope_name,
            props,
            body,
            ret,
        } = self;

        let no_props = props.len() == 1;

        let mut body = body.to_owned();

        // check for components that end ;
        if !is_transparent {
            let ends_semi =
                body.block.stmts.iter().last().and_then(|stmt| match stmt {
                    Stmt::Item(Item::Macro(mac)) => mac.semi_token.as_ref(),
                    _ => None,
                });
            if let Some(semi) = ends_semi {
                proc_macro_error::emit_error!(
                    semi.span(),
                    "A component that ends with a `view!` macro followed by a \
                     semicolon will return (), an empty view. This is usually \
                     an accident, not intentional, so we prevent it. If you’d \
                     like to return (), you can do it it explicitly by \
                     returning () as the last item from the component."
                );
            }
        }

        body.sig.ident = format_ident!("__{}", body.sig.ident);
        #[allow(clippy::redundant_clone)] // false positive
        let body_name = body.sig.ident.clone();

        let (impl_generics, generics, where_clause) =
            body.sig.generics.split_for_impl();

        let lifetimes = body.sig.generics.lifetimes();

        let props_name = format_ident!("{name}Props");
        let props_builder_name = format_ident!("{name}PropsBuilder");
        let trace_name = format!("<{name} />");

        let prop_builder_fields = prop_builder_fields(vis, props);

        let prop_names = prop_names(props);

        let builder_name_doc = LitStr::new(
            &format!("Props for the [`{name}`] component."),
            name.span(),
        );

        let component_fn_prop_docs = generate_component_fn_prop_docs(props);

        let (tracing_instrument_attr, tracing_span_expr, tracing_guard_expr) =
            if cfg!(feature = "tracing") {
                (
                    quote! {
                        #[allow(clippy::let_with_type_underscore)]
                        #[cfg_attr(
                            any(debug_assertions, feature="ssr"),
                            ::leptos::leptos_dom::tracing::instrument(level = "info", name = #trace_name, skip_all)
                        )]
                    },
                    quote! {
                        let span = ::leptos::leptos_dom::tracing::Span::current();
                    },
                    quote! {
                        #[cfg(debug_assertions)]
                        let _guard = span.entered();
                    },
                )
            } else {
                (quote! {}, quote! {}, quote! {})
            };

        let component = if *is_transparent {
            quote! {
                #body_name(#scope_name, #prop_names)
            }
        } else {
            quote! {
                ::leptos::leptos_dom::Component::new(
                    stringify!(#name),
                    move |cx| {
                        #tracing_guard_expr

                        #body_name(cx, #prop_names)
                    }
                )
            }
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
            quote! {
                let #props_name {
                    #prop_names
                } = props;
            }
        };

        let into_view = if no_props {
            quote! {
                impl #impl_generics ::leptos::IntoView for #props_name #generics #where_clause {
                    fn into_view(self, cx: ::leptos::Scope) -> ::leptos::View {
                        #name(cx).into_view(cx)
                    }
                }
            }
        } else {
            quote! {
                impl #impl_generics ::leptos::IntoView for #props_name #generics #where_clause {
                    fn into_view(self, cx: ::leptos::Scope) -> ::leptos::View {
                        #name(cx, self).into_view(cx)
                    }
                }
            }
        };

        let output = quote! {
            #[doc = #builder_name_doc]
            #[doc = ""]
            #docs
            #component_fn_prop_docs
            #[derive(::leptos::typed_builder::TypedBuilder)]
            #[builder(doc)]
            #vis struct #props_name #impl_generics #where_clause {
                #prop_builder_fields
            }

            impl #impl_generics ::leptos::Props for #props_name #generics #where_clause {
                type Builder = #props_builder_name #generics;
                fn builder() -> Self::Builder {
                    #props_name::builder()
                }
            }

            #into_view

            #docs
            #component_fn_prop_docs
            #[allow(non_snake_case, clippy::too_many_arguments)]
            #tracing_instrument_attr
            #vis fn #name #impl_generics (
                #[allow(unused_variables)]
                #scope_name: ::leptos::Scope,
                #props_arg
            ) #ret #(+ #lifetimes)*
            #where_clause
            {
                #body

                #destructure_props

                #tracing_span_expr

                #component
            }
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

        let name = if let Pat::Ident(i) = *typed.pat {
            i
        } else {
            abort!(
                typed.pat,
                "only `prop: bool` style types are allowed within the \
                 `#[component]` macro"
            );
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
        const RUST_START: &str =
            "# ::leptos::create_scope(::leptos::create_runtime(), |cx| {";
        const RUST_END: &str = "# }).dispose();";
        const RSX_START: &str = "# ::leptos::view! {cx,";
        const RSX_END: &str = "# };}).dispose();";

        // Seperated out of chain to allow rustfmt to work
        let map = |(doc, span): (String, Span)| {
            doc.lines()
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
                            quotes = trimmed_doc[..view].to_owned();
                            quote_ws = leading_ws.to_owned();
                            let rust_options = &trimmed_doc
                                [view + "view".len()..]
                                .trim_start();
                            vec![
                                format!("{leading_ws}{quotes}{rust_options}"),
                                format!("{leading_ws}{RUST_START}"),
                            ]
                        }
                        ViewCodeFenceState::Rust if trimmed_doc == quotes => {
                            view_code_fence_state = ViewCodeFenceState::Outside;
                            vec![
                                format!("{leading_ws}{RUST_END}"),
                                doc.to_owned(),
                            ]
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
                attrs.push((format!("{quote_ws}{RUST_END}"), Span::call_site()))
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

#[derive(Clone, Debug, AttributeDerive)]
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
}

struct TypedBuilderOpts {
    default: bool,
    default_with_value: Option<syn::Expr>,
    strip_option: bool,
    into: bool,
}

impl TypedBuilderOpts {
    fn from_opts(opts: &PropOpt, is_ty_option: bool) -> Self {
        Self {
            default: opts.optional || opts.optional_no_strip,
            default_with_value: opts.default.clone(),
            strip_option: opts.strip_option || opts.optional && is_ty_option,
            into: opts.into,
        }
    }
}

impl ToTokens for TypedBuilderOpts {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default_code=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        let strip_option = if self.strip_option {
            quote! { strip_option, }
        } else {
            quote! {}
        };

        let into = if self.into {
            quote! { into, }
        } else {
            quote! {}
        };

        let setter = if !strip_option.is_empty() || !into.is_empty() {
            quote! { setter(#strip_option #into) }
        } else {
            quote! {}
        };

        let output = quote! { #[builder(#default #setter)] };

        tokens.append_all(output);
    }
}

fn prop_builder_fields(vis: &Visibility, props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter(|Prop { ty, .. }| !is_valid_scope_type(ty))
        .map(|prop| {
            let Prop {
                docs,
                name,
                prop_opts,
                ty,
            } = prop;

            let builder_attrs =
                TypedBuilderOpts::from_opts(prop_opts, is_option(ty));

            let builder_docs = prop_to_doc(prop, PropDocStyle::Inline);

            // Children won't need documentation in many cases
            let allow_missing_docs = if name.ident == "children" {
                quote!(#[allow(missing_docs)])
            } else {
                quote!()
            };

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                #allow_missing_docs
                #vis #name: #ty,
            }
        })
        .collect()
}

fn prop_names(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter(|Prop { ty, .. }| !is_valid_scope_type(ty))
        .map(|Prop { name, .. }| quote! { #name, })
        .collect()
}

fn generate_component_fn_prop_docs(props: &[Prop]) -> TokenStream {
    let required_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            !(prop_opts.optional || prop_opts.optional_no_strip)
        })
        .map(|p| prop_to_doc(p, PropDocStyle::List))
        .collect::<TokenStream>();

    let optional_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            prop_opts.optional || prop_opts.optional_no_strip
        })
        .map(|p| prop_to_doc(p, PropDocStyle::List))
        .collect::<TokenStream>();

    let required_prop_docs = if !required_prop_docs.is_empty() {
        quote! {
            #[doc = "# Required Props"]
            #required_prop_docs
        }
    } else {
        quote! {}
    };

    let optional_prop_docs = if !optional_prop_docs.is_empty() {
        quote! {
            #[doc = "# Optional Props"]
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
                    format!("- **{}**: [`{pretty_ty}`]", quote!(#name))
                } else {
                    format!(
                        "- **{}**: [`impl Into<{pretty_ty}>`]({pretty_ty})",
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

fn is_valid_scope_type(ty: &Type) -> bool {
    [
        parse_quote!(Scope),
        parse_quote!(leptos::Scope),
        parse_quote!(::leptos::Scope),
    ]
    .iter()
    .any(|test| ty == test)
}

fn is_valid_into_view_return_type(ty: &ReturnType) -> bool {
    [
        parse_quote!(-> impl IntoView),
        parse_quote!(-> impl leptos::IntoView),
        parse_quote!(-> impl ::leptos::IntoView),
    ]
    .iter()
    .any(|test| ty == test)
}
