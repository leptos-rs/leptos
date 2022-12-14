use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, Attribute, FnArg, ItemFn, Lit, LitStr, Meta, MetaList,
    MetaNameValue, NestedMeta, Pat, PatIdent, Path, ReturnType, Type, TypePath, Visibility,
};

pub struct Model {
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
        item.attrs
            .drain_filter(|attr| attr.path == parse_quote!(doc) || attr.path == parse_quote!(prop));
        item.sig.inputs.iter_mut().for_each(|arg| {
            if let FnArg::Typed(ty) = arg {
                ty.attrs.drain_filter(|attr| {
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

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            docs,
            vis,
            name,
            scope_name,
            props,
            body,
            ret,
        } = self;

        let (impl_generics, generics, where_clause) = body.sig.generics.split_for_impl();

        let props_name = format_ident!("{name}Props");

        let prop_builder_fields = prop_builder_fields(props);

        let prop_names = prop_names(props);

        let name_stringified = LitStr::new(&name.to_string(), name.span());

        let component_fn_prop_docs = generate_component_fn_prop_docs(props);

        let output = quote! {
            #[doc = "Props for the [`"]
            #[doc = #name_stringified]
            #[doc = "`] component"]
            #[derive(leptos::TypedBuilder)]
            #[builder(doc)]
            #vis struct #props_name #generics #where_clause {
                #prop_builder_fields
            }

            #docs
            #component_fn_prop_docs
            #[allow(non_snake_case, clippy::too_many_arguments)]
            #vis fn #name #generics (#scope_name: Scope, props: #props_name #generics) #ret
            #where_clause
            {
                #body

                let #props_name {
                    #prop_names
                } = props;

                leptos::Component::new(
                    stringify!(#name),
                    move |cx| #name(cx, #prop_names)
                )
            }
        };

        tokens.append_all(output)
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

    fn typed_builder(&self) -> TokenStream {
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
            quote! { #[builder(setter(doc = #doc_str))] }
        } else {
            quote! {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PropOpt {
    Optional,
    OptionalNoStrip,
    StripOption,
    Into,
}

impl PropOpt {
    fn from_attribute(attr: &Attribute) -> Option<HashSet<Self>> {
        const ABORT_OPT_MESSAGE: &str = "only `optional`, \
                                         `optional_no_strip`, \
                                         `strip_option`, and `into` are \
                                         allowed as arguments to `#[prop()]`";

        if attr.path != parse_quote!(prop) {
            return None;
        }

        if let Meta::List(MetaList { nested, .. }) = attr.parse_meta().ok()? {
            Some(
                nested
                    .iter()
                    .map(|opt| {
                        if let NestedMeta::Meta(Meta::Path(opt)) = opt {
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
                        } else {
                            abort!(opt, ABORT_OPT_MESSAGE,);
                        }
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
    strip_option: bool,
    into: bool,
}

impl TypedBuilderOpts {
    fn from_opts(opts: &HashSet<PropOpt>, is_ty_option: bool) -> Self {
        Self {
            default: opts.contains(&PropOpt::Optional) || opts.contains(&PropOpt::OptionalNoStrip),
            strip_option: opts.contains(&PropOpt::StripOption)
                || (opts.contains(&PropOpt::Optional) && is_ty_option),
            into: opts.contains(&PropOpt::Into),
        }
    }
}

impl ToTokens for TypedBuilderOpts {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if self.default {
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

fn prop_builder_fields(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter(|Prop { ty, .. }| *ty != parse_quote!(Scope))
        .map(
            |Prop {
                 docs,
                 name,
                 prop_opts,
                 ty,
             }| {
                let builder_attrs = TypedBuilderOpts::from_opts(prop_opts, is_option(ty));

                let builder_docs = docs.typed_builder();

                quote! {
                   #docs
                   #builder_docs
                   #builder_attrs
                   pub #name: #ty,
                }
            },
        )
        .collect()
}

fn component_args(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .map(|Prop { name, ty, .. }| quote! { #name: #ty, })
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
    let header = quote! { #[doc = "# Props"] };

    let prop_docs = props
        .iter()
        .map(|Prop { docs, name, ty, .. }| {
            let arg_ty_doc = LitStr::new(
                &format!("- **{}**: [`{}`]", quote!(#name), quote!(#ty)),
                name.ident.span(),
            );

            let arg_user_docs = docs.padded();

            quote! {
                #[doc = #arg_ty_doc]
                #arg_user_docs
            }
        })
        .collect::<TokenStream>();

    quote! {
        #header
        #prop_docs
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
