use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error2::abort;
use quote::{format_ident, quote};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};
use syn::{parse_macro_input, parse_quote, ItemFn, ReturnType, Stmt};

pub fn lazy_impl(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let name = if !args.is_empty() {
        Some(parse_macro_input!(args as syn::Ident))
    } else {
        None
    };

    let fun = syn::parse::<ItemFn>(s).unwrap_or_else(|e| {
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
        let mut fun = fun;
        let mut return_wrapper = None;
        if was_async {
            fun.sig.asyncness = None;
            let ty = match &fun.sig.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };
            let sync_output: ReturnType = parse_quote! {
                -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #ty> + ::std::marker::Send + ::std::marker::Sync>>
            };
            let async_output = mem::replace(&mut fun.sig.output, sync_output);
            let stmts = mem::take(&mut fun.block.stmts);
            fun.block.stmts.push(Stmt::Expr(parse_quote! {
                ::std::boxed::Box::pin(::leptos::__reexports::send_wrapper::SendWrapper::new(async move {
                    #( #stmts )*
                }))
            }, None));
            return_wrapper = Some(quote! {
                return_wrapper(let future = _; { future.await } #async_output),
            });
        }
        let preload_name = format_ident!("__preload_{}", fun.sig.ident);

        quote! {
            #[::leptos::wasm_split::wasm_split(
                #unique_name,
                wasm_split_path = ::leptos::wasm_split,
                preload(#[doc(hidden)] #[allow(non_snake_case)] #preload_name),
                #return_wrapper
            )]
            #fun
        }
    } else {
        let mut fun = fun;
        if !was_async {
            fun.sig.asyncness = Some(Default::default());
        }

        let statements = &mut fun.block.stmts;
        let old_statements = mem::take(statements);
        statements.push(parse_quote! {
            ::leptos::prefetch_lazy_fn_on_server(#unique_name_str);
        });
        statements.extend(old_statements);
        quote! { #fun }
    }
    .into()
}
