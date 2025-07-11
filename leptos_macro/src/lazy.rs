use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error2::abort;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, ItemFn};

pub fn lazy_impl(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let name =
        (!args.is_empty()).then(|| parse_macro_input!(args as syn::Ident));

    let fun = syn::parse::<ItemFn>(s).unwrap_or_else(|e| {
        abort!(e.span(), "`lazy` can only be used on a function")
    });
    if fun.sig.asyncness.is_none() {
        abort!(
            fun.sig.asyncness.span(),
            "`lazy` can only be used on an async function"
        )
    }

    let converted_name = name.unwrap_or_else(|| {
        Ident::new(
            &fun.sig.ident.to_string().to_case(Case::Snake),
            fun.sig.ident.span(),
        )
    });

    let is_wasm = cfg!(feature = "csr") || cfg!(feature = "hydrate");
    if is_wasm {
        quote! {
            #[::leptos::wasm_split::wasm_split(#converted_name)]
            #fun
        }
    } else {
        quote! { #fun }
    }
    .into()
}
