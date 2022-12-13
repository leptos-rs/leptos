use proc_macro2::{Ident, TokenStream};
use syn::{
    parse::Parse, parse_quote, Attribute, FnArg, ItemFn, Pat, PatIdent, ReturnType, Type,
    Visibility,
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
        let item = ItemFn::parse(input)?;

        let docs = Docs::new(&item.attrs);

        Ok(Self {
            docs,
            vis: item.vis.clone(),
            name: item.sig.ident.clone(),
            props: item.sig.inputs.clone().into_iter().map(Prop::new).collect(),
            ret: item.sig.output.clone(),
            body: item,
        })
    }
}

impl Into<TokenStream> for Model {
    fn into(self) -> TokenStream {
        let Self {
            docs,
            vis,
            name,
            props,
            body,
            ret,
        } = self;

        quote!(
            #vis #name() #ret {
                todo!();
            }
        );

        todo!()
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

struct Docs(pub Vec<Attribute>);

impl Docs {
    fn new(attrs: &[Attribute]) -> Self {
        let attrs = attrs
            .iter()
            .filter(|attr| attr.path == parse_quote!(doc))
            .cloned()
            .collect();

        Self(attrs)
    }
}
