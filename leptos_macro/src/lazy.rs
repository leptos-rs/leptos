use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error2::abort;
use quote::quote;
use syn::{spanned::Spanned, ItemFn};

pub fn lazy_impl(
    _args: proc_macro::TokenStream,
    s: TokenStream,
) -> TokenStream {
    let fun = syn::parse::<ItemFn>(s).unwrap_or_else(|e| {
        abort!(e.span(), "`lazy` can only be used on a function")
    });
    if fun.sig.asyncness.is_none() {
        abort!(
            fun.sig.asyncness.span(),
            "`lazy` can only be used on an async function"
        )
    }

    let converted_name = Ident::new(
        &fun.sig.ident.to_string().to_case(Case::Snake),
        fun.sig.ident.span(),
    );

    quote! {
        #[cfg_attr(feature = "split", wasm_split::wasm_split(#converted_name))]
        #fun
    }
    .into()
}
