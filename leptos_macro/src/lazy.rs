use crate::stable_hash::fnv1a_64;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro_error2::abort;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::mem;
use syn::{
    ItemFn, Path, ReturnType, Stmt, Token, parse::Parse, parse_macro_input,
    parse_quote, punctuated::Punctuated,
};

fn preload_name(ident: &Ident) -> Ident {
    format_ident!("__preload_{}", ident)
}

pub fn lazy_impl(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    // Optional, comma-separated args: an explicit module name (a bare ident)
    // and/or the `fallible` flag. `fallible` forwards to
    // `#[wasm_split(.., fallible)]`, so the annotated function must return
    // `Result<_, E: From<SplitLoaderError>>` and a failed chunk load is
    // surfaced as `Err` instead of panicking.
    let parsed_args = parse_macro_input!(
        args with Punctuated::<Ident, Token![,]>::parse_terminated
    );
    let mut name = None;
    let mut fallible = false;
    for ident in parsed_args {
        if ident == "fallible" {
            fallible = true;
        } else if name.is_none() {
            name = Some(ident);
        } else {
            abort!(ident.span(), "unexpected `lazy` argument");
        }
    }

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
        let location = format!(
            "{}:{}:{}",
            span.line(),
            span.start().column(),
            span.file()
        );
        let hash = fnv1a_64(location.as_bytes());

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
        let preload_name = preload_name(&fun.sig.ident);
        let fallible_opt = if fallible {
            quote! { fallible, }
        } else {
            quote! {}
        };

        quote! {
            #[::leptos::wasm_split::wasm_split(
                #unique_name,
                wasm_split_path = ::leptos::wasm_split,
                #fallible_opt
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

struct LazyPath(Path);

impl Parse for LazyPath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self(Path::parse_mod_style(input)?))
    }
}

pub fn lazy_preload_impl(s: proc_macro::TokenStream) -> TokenStream {
    let LazyPath(mut path) = syn::parse::<LazyPath>(s).unwrap_or_else(|e| {
        abort!(
            e.span(),
            "`lazy_preload` only takes a function path as argument"
        )
    });
    let last_segment = path.segments.last_mut().unwrap_or_else(|| {
        abort_call_site!(
            "`lazy_preload` needs a path ending with an identifier"
        )
    });
    last_segment.ident = preload_name(&last_segment.ident);

    let preload_call = if cfg!(feature = "hydrate") {
        quote! {
            ::leptos::task::spawn_local(async move {

                #path().await;
                set_loaded.set(true);
            });
        }
    } else {
        quote! {}
    };

    quote! {{
        use ::leptos::prelude::Set;

        let (loaded, set_loaded) = ::leptos::prelude::signal(false);
        #preload_call
        loaded
    }}
    .into()
}
