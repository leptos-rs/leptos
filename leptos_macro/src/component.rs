use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, ToTokens, TokenStreamExt};
use syn::{
    parse::Parse, parse_quote, Attribute, FnArg, ItemFn, Lit, LitStr, Meta, MetaNameValue, Pat,
    PatIdent, Path, ReturnType, Type, TypePath, Visibility,
};

pub struct Model {
    docs: Docs,
    vis: Visibility,
    name: Ident,
    props: Vec<Prop>,
    body: ItemFn,
    ret: ReturnType,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut item = ItemFn::parse(input)?;

        let docs = Docs::new(&item.attrs);

        let props = item.sig.inputs.clone().into_iter().map(Prop::new).collect();

        // We need to remove the `#[doc = ""]` and `#[builder(_)]`
        // attrs from the function signature
        item.attrs.drain_filter(|attr| {
            attr.path == parse_quote!(doc) || attr.path == parse_quote!(builder)
        });
        item.sig.inputs.iter_mut().for_each(|arg| {
            if let FnArg::Typed(ty) = arg {
                ty.attrs.drain_filter(|attr| {
                    attr.path == parse_quote!(doc) || attr.path == parse_quote!(builder)
                });
            }
        });

        Ok(Self {
            docs,
            vis: item.vis.clone(),
            name: item.sig.ident.clone(),
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

        let body = &body.block;

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
            #[allow(non_snake_case)]
            #vis fn #name #generics (cx: Scope, props: #props_name #generics) #ret
            #where_clause
            {
                let #props_name {
                    #prop_names
                } = props;

                leptos::Component::new(
                    stringify!(#name),
                    move |cx| {#body}
                )
            }
        };

        tokens.append_all(output)
    }
}

struct Prop {
    pub docs: Docs,
    pub typed_builder_attrs: Vec<Attribute>,
    pub name: PatIdent,
    pub ty: Type,
}

impl Prop {
    fn new(arg: FnArg) -> Self {
        let typed = if let FnArg::Typed(ty) = arg {
            ty
        } else {
            abort!(arg, "receiver not allowed in `fn`");
        };

        let typed_builder_attrs = typed
            .attrs
            .iter()
            .filter(|attr| attr.path == parse_quote!(builder))
            .cloned()
            .collect();

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
            typed_builder_attrs,
            name,
            ty: *typed.ty,
        }
    }
}

#[derive(Clone)]
struct Docs(pub Vec<Attribute>);

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

fn prop_builder_fields(props: &[Prop]) -> TokenStream {
    props
        .iter()
        .filter(|Prop { ty, .. }| *ty != parse_quote!(Scope))
        .map(
            |Prop {
                 docs,
                 name,
                 typed_builder_attrs,
                 ty,
             }| {
                let typed_builder_attrs = typed_builder_attrs
                    .iter()
                    .map(|attr| quote! { #attr })
                    .collect::<TokenStream>();

                let builder_docs = docs.typed_builder();

                let builder_attr = if is_option(&ty) && typed_builder_attrs.is_empty() {
                    quote! { #[builder(default, setter(strip_option))] }
                } else {
                    quote! {}
                };

                quote! {
                   #docs
                   #builder_docs
                   #typed_builder_attrs
                   #builder_attr
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
