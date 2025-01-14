use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro_error2::abort;
use quote::quote;
use syn::{
    spanned::Spanned, Block, ImplItem, ItemFn, ItemImpl, Path, Type, TypePath,
};

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

pub fn lazy_route_impl(
    _args: proc_macro::TokenStream,
    s: TokenStream,
) -> TokenStream {
    let mut im = syn::parse::<ItemImpl>(s).unwrap_or_else(|e| {
        abort!(e.span(), "`lazy_route` can only be used on an `impl` block")
    });
    if im.trait_.is_none() {
        abort!(
            im.span(),
            "`lazy_route` can only be used on an `impl LazyRoute for ...` \
             block"
        )
    }

    let self_ty = im.self_ty.clone();
    let ty_name_to_snake = match &*self_ty {
        Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) => segments.last().unwrap().ident.to_string(),
        _ => abort!(self_ty.span(), "only path types are supported"),
    };
    let lazy_view_ident = Ident::new(&ty_name_to_snake, im.self_ty.span());

    let item = im.items.iter_mut().find_map(|item| match item {
        ImplItem::Fn(inner) => {
            if inner.sig.ident.to_string().as_str() == "view" {
                Some(inner)
            } else {
                None
            }
        }
        _ => None,
    });
    match item {
        None => abort!(im.span(), "must contain a fn called `view`"),
        Some(fun) => {
            let body = fun.block.clone();
            let new_block = quote! {{
                    #[cfg_attr(feature = "split", wasm_split::wasm_split(#lazy_view_ident))]
                    async fn view(this: #self_ty) -> ::leptos::prelude::AnyView {
                        #body
                    }

                    view(self).await
            }};
            let block =
                syn::parse::<Block>(new_block.into()).unwrap_or_else(|e| {
                    abort!(
                        e.span(),
                        "`lazy_route` can only be used on an `impl` block"
                    )
                });
            fun.block = block;
        }
    }

    quote! { #im }.into()
}
