extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Token,
};

struct MemoMacroInput {
    root: syn::Ident,
    path: Punctuated<syn::Member, Token![.]>,
}

impl Parse for MemoMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let root: syn::Ident = input.parse()?;
        input.parse::<Token![.]>()?;
        // do not accept trailing punctuation
        let path: Punctuated<syn::Member, Token![.]> =
            Punctuated::parse_separated_nonempty(input)?;

        if path.is_empty() {
            return Err(input.error("expected identifier"));
        }

        if !input.is_empty() {
            return Err(input.error("unexpected token"));
        }

        Ok(Self { root, path })
    }
}

impl ToTokens for MemoMacroInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let root = &self.root;
        let path = &self.path;

        tokens.extend(quote! {
            ::leptos::reactive::computed::Memo::new(
                move |_| {
                    use ::leptos::reactive::traits::With;
                    #root.with(|st: _| st.#path.clone())
                }
            )
        })
    }
}

pub fn memo_impl(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as MemoMacroInput);
    input.into_token_stream().into()
}
