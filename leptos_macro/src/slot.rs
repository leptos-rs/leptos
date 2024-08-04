use crate::component::{
    convert_from_snake_case, drain_filter, is_option, unwrap_option, Docs,
};
use attribute_derive::FromAttr;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, Field, ItemStruct, LitStr, Meta, Type,
    Visibility,
};

pub struct Model {
    docs: Docs,
    vis: Visibility,
    name: Ident,
    props: Vec<Prop>,
    body: ItemStruct,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut item = ItemStruct::parse(input)?;

        let docs = Docs::new(&item.attrs);

        let props = item
            .fields
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
        item.fields.iter_mut().for_each(|arg| {
            drain_filter(&mut arg.attrs, |attr| match &attr.meta {
                Meta::NameValue(attr) => attr.path == parse_quote!(doc),
                Meta::List(attr) => attr.path == parse_quote!(prop),
                _ => false,
            });
        });

        Ok(Self {
            docs,
            vis: item.vis.clone(),
            name: convert_from_snake_case(&item.ident),
            props,
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
            props,
            body,
        } = self;

        let (_, generics, where_clause) = body.generics.split_for_impl();

        let prop_builder_fields = prop_builder_fields(vis, props);
        let prop_docs = generate_prop_docs(props);
        let builder_name_doc = LitStr::new(
            &format!("Props for the [`{name}`] slot."),
            name.span(),
        );

        let output = quote! {
            #[doc = #builder_name_doc]
            #[doc = ""]
            #docs
            #prop_docs
            #[derive(::leptos::typed_builder_macro::TypedBuilder)]
            #[builder(doc, crate_module_path=::leptos::typed_builder)]
            #vis struct #name #generics #where_clause {
                #prop_builder_fields
            }

            impl #generics From<#name #generics> for Vec<#name #generics> #where_clause {
                fn from(value: #name #generics) -> Self {
                    vec![value]
                }
            }

            /*impl #impl_generics ::leptos::Props for #name #generics #where_clause {
                type Builder = #builder_name #generics;
                fn builder() -> Self::Builder {
                    #name::builder()
                }
            }

            impl #impl_generics ::leptos::DynAttrs for #name #generics #where_clause {
                fn dyn_attrs(mut self, v: Vec<(&'static str, ::leptos::Attribute)>) -> Self {
                    #dyn_attrs_props
                    self
                }
            }*/
        };

        tokens.append_all(output)
    }
}

struct Prop {
    docs: Docs,
    prop_opts: PropOpt,
    name: Ident,
    ty: Type,
}

impl Prop {
    fn new(arg: Field) -> Self {
        let prop_opts =
            PropOpt::from_attributes(&arg.attrs).unwrap_or_else(|e| {
                // TODO: replace with `.unwrap_or_abort()` once https://gitlab.com/CreepySkeleton/proc-macro-error/-/issues/17 is fixed
                abort!(e.span(), e.to_string());
            });

        let name = if let Some(i) = arg.ident {
            i
        } else {
            abort!(
                arg.ident,
                "only `prop: bool` style types are allowed within the \
                 `#[slot]` macro"
            );
        };

        Self {
            docs: Docs::new(&arg.attrs),
            prop_opts,
            name,
            ty: arg.ty,
        }
    }
}

#[derive(Clone, Debug, FromAttr)]
#[attribute(ident = prop)]
struct PropOpt {
    #[attribute(conflicts = [optional_no_strip, strip_option])]
    pub optional: bool,
    #[attribute(conflicts = [optional, strip_option])]
    pub optional_no_strip: bool,
    #[attribute(conflicts = [optional, optional_no_strip])]
    pub strip_option: bool,
    #[attribute(example = "5 * 10")]
    pub default: Option<syn::Expr>,
    pub into: bool,
    pub attrs: bool,
}

struct TypedBuilderOpts {
    default: bool,
    default_with_value: Option<syn::Expr>,
    strip_option: bool,
    into: bool,
}

impl TypedBuilderOpts {
    pub fn from_opts(opts: &PropOpt, is_ty_option: bool) -> Self {
        Self {
            default: opts.optional || opts.optional_no_strip || opts.attrs,
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

        let output = if !default.is_empty() || !setter.is_empty() {
            quote! { #[builder(#default #setter)] }
        } else {
            quote! {}
        };

        tokens.append_all(output);
    }
}

fn prop_builder_fields(vis: &Visibility, props: &[Prop]) -> TokenStream {
    props
        .iter()
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

            quote! {
                #docs
                #builder_docs
                #builder_attrs
                #vis #name: #ty,
            }
        })
        .collect()
}

fn generate_prop_docs(props: &[Prop]) -> TokenStream {
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
                    format!("- **{}**: [`{}`]", quote!(#name), pretty_ty)
                } else {
                    format!(
                        "- **{}**: `impl`[`Into<{}>`]",
                        quote!(#name),
                        pretty_ty
                    )
                },
                name.span(),
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
                name.span(),
            );

            quote! {
                #[builder(setter(doc = #arg_ty_doc))]
            }
        }
    }
}
