use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, ToTokens, TokenStreamExt};
use std::collections::HashSet;
use syn::{
    parse::Parse, parse_quote, AngleBracketedGenericArguments, Attribute, FnArg, GenericArgument,
    ItemFn, LitStr, Meta, MetaList, MetaNameValue, NestedMeta, Pat, PatIdent, Path, PathArguments,
    ReturnType, Type, TypePath, Visibility,
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
        } else if props[0].ty != parse_quote!(Scope) {
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
        drain_filter(&mut item.attrs, |attr| {
            attr.path == parse_quote!(doc) || attr.path == parse_quote!(prop)
        });
        item.sig.inputs.iter_mut().for_each(|arg| {
            if let FnArg::Typed(ty) = arg {
                drain_filter(&mut ty.attrs, |attr| {
                    attr.path == parse_quote!(doc) || attr.path == parse_quote!(prop)
                });
            }
        });

        // Make sure return type is correct
        if item.sig.output != parse_quote!(-> impl IntoView) {
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
            name: item.sig.ident.clone(),
            scope_name,
            props,
            ret: item.sig.output.clone(),
            body: item,
        })
    }
}

// implemented manually because Vec::drain_filter is nightly only
// follows std recommended parallel
fn drain_filter<T>(vec: &mut Vec<T>, mut some_predicate: impl FnMut(&mut T) -> bool) {
    let mut i = 0;
    while i < vec.len() {
        if some_predicate(&mut vec[i]) {
            _ = vec.remove(i);
        } else {
            i += 1;
        }
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

        let mut body = body.to_owned();

        body.sig.ident = format_ident!("__{}", body.sig.ident);
        let body_name = body.sig.ident.clone();

        let (_, generics, where_clause) = body.sig.generics.split_for_impl();
        let lifetimes = body.sig.generics.lifetimes();

        let props_name = format_ident!("{name}Props");
        let trace_name = format!("<{name} />");

        let prop_builder_fields = prop_builder_fields(vis, props);

        let prop_names = prop_names(props);

        let builder_name_doc =
            LitStr::new(&format!("Props for the [`{name}`] component."), name.span());

        let component_fn_prop_docs = generate_component_fn_prop_docs(props);

        let (tracing_instrument_attr, tracing_span_expr, tracing_guard_expr) = if cfg!(
            feature = "tracing"
        ) {
            (
                quote! {
                    #[cfg_attr(
                        debug_assertions,
                        ::leptos::leptos_dom::tracing::instrument(level = "trace", name = #trace_name, skip_all)
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
                #body_name(cx, #prop_names)
            }
        } else {
            quote! {
                ::leptos::Component::new(
                    stringify!(#name),
                    move |cx| {
                        #tracing_guard_expr

                        #body_name(cx, #prop_names)
                    }
                )
            }
        };

        let output = quote! {
            #[doc = #builder_name_doc]
            #[doc = ""]
            #docs
            #component_fn_prop_docs
            #[derive(::leptos::typed_builder::TypedBuilder)]
            #[builder(doc)]
            #vis struct #props_name #generics #where_clause {
                #prop_builder_fields
            }

            #docs
            #component_fn_prop_docs
            #[allow(non_snake_case, clippy::too_many_arguments)]
            #tracing_instrument_attr
            #vis fn #name #generics (
                #[allow(unused_variables)]
                #scope_name: Scope,
                props: #props_name #generics
            ) #ret #(+ #lifetimes)*
            #where_clause
            {
                #body

                let #props_name {
                    #prop_names
                } = props;

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
    prop_opts: HashSet<PropOpt>,
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

        let prop_opts = typed
            .attrs
            .iter()
            .enumerate()
            .filter_map(|(i, attr)| PropOpt::from_attribute(attr).map(|opt| (i, opt)))
            .fold(HashSet::new(), |mut acc, cur| {
                // Make sure opts aren't repeated
                if acc.intersection(&cur.1).next().is_some() {
                    abort!(typed.attrs[cur.0], "`#[prop]` options are repeated");
                }

                acc.extend(cur.1);

                acc
            });

        // Make sure conflicting options are not present
        if prop_opts.contains(&PropOpt::Optional) && prop_opts.contains(&PropOpt::OptionalNoStrip) {
            abort!(
                typed,
                "`optional` and `optional_no_strip` options are mutually exclusive"
            );
        } else if prop_opts.contains(&PropOpt::Optional)
            && prop_opts.contains(&PropOpt::StripOption)
        {
            abort!(
                typed,
                "`optional` and `strip_option` options are mutually exclusive"
            );
        } else if prop_opts.contains(&PropOpt::OptionalNoStrip)
            && prop_opts.contains(&PropOpt::StripOption)
        {
            abort!(
                typed,
                "`optional_no_strip` and `strip_option` options are mutually exclusive"
            );
        }

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
struct Docs(Vec<Attribute>);

impl ToTokens for Docs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = self
            .0
            .iter()
            .map(|attr| attr.to_token_stream())
            .collect::<TokenStream>();

        tokens.append_all(s);
    }
}

impl Docs {
    fn new(attrs: &[Attribute]) -> Self {
        let attrs = attrs
            .iter()
            .filter(|attr| attr.path == parse_quote!(doc))
            .cloned()
            .collect();

        Self(attrs)
    }

    fn padded(&self) -> TokenStream {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, attr)| {
                if let Meta::NameValue(MetaNameValue { lit: doc, .. }) = attr.parse_meta().unwrap()
                {
                    let doc_str = quote!(#doc);

                    // We need to remove the leading and trailing `"`"
                    let mut doc_str = doc_str.to_string();
                    doc_str.pop();
                    doc_str.remove(0);

                    let doc_str = if idx == 0 {
                        format!("    - {doc_str}")
                    } else {
                        format!("      {doc_str}")
                    };

                    let docs = LitStr::new(&doc_str, doc.span());

                    if !doc_str.is_empty() {
                        quote! { #[doc = #docs] }
                    } else {
                        quote! {}
                    }
                } else {
                    unreachable!()
                }
            })
            .collect()
    }

    fn typed_builder(&self) -> String {
        #[allow(unstable_name_collisions)]
        let doc_str = self
            .0
            .iter()
            .map(|attr| {
                if let Meta::NameValue(MetaNameValue { lit: doc, .. }) = attr.parse_meta().unwrap()
                {
                    let mut doc_str = quote!(#doc).to_string();

                    // Remove the leading and trailing `"`
                    doc_str.pop();
                    doc_str.remove(0);

                    doc_str
                } else {
                    unreachable!()
                }
            })
            .intersperse("\n".to_string())
            .collect::<String>();

        if doc_str.chars().filter(|c| *c != '\n').count() != 0 {
            format!("\n\n{doc_str}")
        } else {
            String::new()
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum PropOpt {
    Optional,
    OptionalNoStrip,
    OptionalWithDefault(syn::Lit),
    StripOption,
    Into,
}

impl PropOpt {
    fn from_attribute(attr: &Attribute) -> Option<HashSet<Self>> {
        const ABORT_OPT_MESSAGE: &str = "only `optional`, \
                                         `optional_no_strip`, \
                                         `strip_option`, \
                                         `default` and `into` are \
                                         allowed as arguments to `#[prop()]`";

        if attr.path != parse_quote!(prop) {
            return None;
        }

        if let Meta::List(MetaList { nested, .. }) = attr.parse_meta().ok()? {
            Some(
                nested
                    .iter()
                    .map(|opt| match opt {
                        NestedMeta::Meta(Meta::Path(opt)) => {
                            if *opt == parse_quote!(optional) {
                                PropOpt::Optional
                            } else if *opt == parse_quote!(optional_no_strip) {
                                PropOpt::OptionalNoStrip
                            } else if *opt == parse_quote!(strip_option) {
                                PropOpt::StripOption
                            } else if *opt == parse_quote!(into) {
                                PropOpt::Into
                            } else {
                                abort!(
                                    opt,
                                    "invalid prop option";
                                    help = ABORT_OPT_MESSAGE
                                );
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                            path,
                            eq_token: _,
                            lit,
                        })) => {
                            if *path == parse_quote!(default) {
                                PropOpt::OptionalWithDefault(lit.to_owned())
                            } else {
                                abort!(
                                    opt,
                                    "invalid prop option";
                                    help = ABORT_OPT_MESSAGE
                                );
                            }
                        }
                        _ => abort!(opt, ABORT_OPT_MESSAGE,),
                    })
                    .collect(),
            )
        } else {
            abort!(
                attr,
                "the syntax for `#[prop]` is incorrect";
                help = "try `#[prop(optional)]`";
                help = ABORT_OPT_MESSAGE
            );
        }
    }
}

struct TypedBuilderOpts {
    default: bool,
    default_with_value: Option<syn::Lit>,
    strip_option: bool,
    into: bool,
}

impl TypedBuilderOpts {
    fn from_opts(opts: &HashSet<PropOpt>, is_ty_option: bool) -> Self {
        Self {
            default: opts.contains(&PropOpt::Optional) || opts.contains(&PropOpt::OptionalNoStrip),
            default_with_value: opts.iter().find_map(|p| match p {
                PropOpt::OptionalWithDefault(v) => Some(v.to_owned()),
                _ => None,
            }),
            strip_option: opts.contains(&PropOpt::StripOption)
                || (opts.contains(&PropOpt::Optional) && is_ty_option),
            into: opts.contains(&PropOpt::Into),
        }
    }
}

impl ToTokens for TypedBuilderOpts {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if let Some(v) = &self.default_with_value {
            quote! { default=#v, }
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
        .filter(|Prop { ty, .. }| *ty != parse_quote!(Scope))
        .map(|prop| {
            let Prop {
                docs,
                name,
                prop_opts,
                ty,
            } = prop;

            let builder_attrs = TypedBuilderOpts::from_opts(prop_opts, is_option(ty));

            let builder_docs = prop_to_doc(prop, PropDocStyle::Inline);

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                #vis #name: #ty,
            }
        })
        .collect()
}

fn prop_names(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter(|Prop { ty, .. }| *ty != parse_quote!(Scope))
        .map(|Prop { name, .. }| quote! { #name, })
        .collect()
}

fn generate_component_fn_prop_docs(props: &[Prop]) -> TokenStream {
    let required_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            !(prop_opts.contains(&PropOpt::Optional)
                || prop_opts.contains(&PropOpt::OptionalNoStrip))
        })
        .map(|p| prop_to_doc(p, PropDocStyle::List))
        .collect::<TokenStream>();

    let optional_prop_docs = props
        .iter()
        .filter(|Prop { prop_opts, .. }| {
            prop_opts.contains(&PropOpt::Optional) || prop_opts.contains(&PropOpt::OptionalNoStrip)
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

fn is_option(ty: &Type) -> bool {
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

fn unwrap_option(ty: &Type) -> Option<Type> {
    const STD_OPTION_MSG: &str = "make sure you're not shadowing the \
    `std::option::Option` type that is automatically imported from the \
    standard prelude";

    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let [first] = &segments.iter().collect::<Vec<_>>()[..] {
            if first.ident == "Option" {
                if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args, ..
                }) = &first.arguments
                {
                    if let [first] = &args.iter().collect::<Vec<_>>()[..] {
                        if let GenericArgument::Type(ty) = first {
                            Some(ty.clone())
                        } else {
                            abort!(
                                first,
                                "`Option` must be `std::option::Option`";
                                help = STD_OPTION_MSG
                            );
                        }
                    } else {
                        abort!(
                            first,
                            "`Option` must be `std::option::Option`";
                            help = STD_OPTION_MSG
                        );
                    }
                } else {
                    abort!(
                        first,
                        "`Option` must be `std::option::Option`";
                        help = STD_OPTION_MSG
                    );
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
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
    let ty = if (prop_opts.contains(&PropOpt::Optional)
        || prop_opts.contains(&PropOpt::StripOption))
        && is_option(ty)
    {
        unwrap_option(ty).unwrap()
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
                &if !prop_opts.contains(&PropOpt::Into) {
                    format!("- **{}**: [`{}`]", quote!(#name), pretty_ty)
                } else {
                    format!("- **{}**: `impl`[`Into<{}>`]", quote!(#name), pretty_ty)
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
                &if !prop_opts.contains(&PropOpt::Into) {
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
