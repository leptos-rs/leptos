use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error2::abort;
use quote::quote;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};
use syn::{parse_macro_input, ItemFn};

pub fn lazy_impl(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let name = if !args.is_empty() {
        Some(parse_macro_input!(args as syn::Ident))
    } else {
        None
    };

    let mut fun = syn::parse::<ItemFn>(s).unwrap_or_else(|e| {
        abort!(e.span(), "`lazy` can only be used on a function")
    });

    let was_async = fun.sig.asyncness.is_some();

    let converted_name = name.unwrap_or_else(|| {
        Ident::new(
            &fun.sig.ident.to_string().to_case(Case::Snake),
            fun.sig.ident.span(),
        )
    });

    let (unique_name, unique_name_str) = {
        let span = proc_macro::Span::call_site();
        let location = (span.line(), span.start().column(), span.file());

        let mut hasher = DefaultHasher::new();
        location.hash(&mut hasher);
        let hash = hasher.finish();

        let unique_name_str = format!("{converted_name}_{hash}");

        (
            Ident::new(&unique_name_str, converted_name.span()),
            unique_name_str,
        )
    };

    let is_wasm = cfg!(feature = "csr") || cfg!(feature = "hydrate");
    if is_wasm {
        quote! {
            #[::leptos::wasm_split_helpers::wasm_split(
                #unique_name,
                ::leptos::__reexports::send_wrapper
            )]
            #fun
        }
    } else {
        if !was_async {
            fun.sig.asyncness = Some(Default::default());
        }

        let statements = &mut fun.block.stmts;
        let old_statements = mem::take(statements);
        statements.push(
            syn::parse(
                quote! {
                    ::leptos::prefetch_lazy_fn_on_server(#unique_name_str);
                }
                .into(),
            )
            .unwrap(),
        );
        statements.extend(old_statements);
        quote! { #fun }
    }
    .into()
}
