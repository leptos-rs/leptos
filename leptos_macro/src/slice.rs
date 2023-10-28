extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Dot,
    Token, Type,
};

struct SliceMacroInput {
    pub root: Ident,
    pub path: Punctuated<Type, Dot>,
}

impl Parse for SliceMacroInput {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let root = syn::Ident::parse(input)?;
        let _dot = <Token![.]>::parse(input)?;
        let path = input.parse_terminated(Type::parse, Token![.])?;

        if path.is_empty() {
            return Err(syn::Error::new(input.span(), "Expected identifier"));
        }

        if path.trailing_punct() {
            return Err(syn::Error::new(
                input.span(),
                "Unexpected trailing `.`",
            ));
        }

        Ok(SliceMacroInput { root, path })
    }
}

impl From<SliceMacroInput> for TokenStream {
    fn from(val: SliceMacroInput) -> Self {
        let root = val.root;
        let path = val.path;

        quote! {
            ::leptos::create_slice(
                #root,
                |st: &_| st.#path.clone(),
                |st: &mut _, n| st.#path = n
            )
        }
        .into()
    }
}

pub fn slice_impl(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as SliceMacroInput);
    input.into()
}
