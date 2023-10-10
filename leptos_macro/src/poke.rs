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

struct PokeMacroInput {
    pub root: Ident,
    pub path: Punctuated<Type, Dot>,
}

impl Parse for PokeMacroInput {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let root = syn::Ident::parse(input)?;
        let _dot = <Token![.]>::parse(input)?;
        let path = input.parse_terminated(Type::parse, Token![.])?;

        Ok(PokeMacroInput { root, path })
    }
}

impl From<PokeMacroInput> for TokenStream {
    fn from(val: PokeMacroInput) -> Self {
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

pub fn poke_impl(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as PokeMacroInput);
    input.into()
}
