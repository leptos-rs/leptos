use digest::Digest;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Comma,
    Ident, ItemFn, Path, Result, ReturnType, Signature,
};

struct WasmSplitArgs {
    module_ident: Ident,
    _comma: Option<Comma>,
    send_wrapper_path: Option<Path>,
}

impl Parse for WasmSplitArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let module_ident = input.parse()?;
        let _comma = input.parse().ok();
        let send_wrapper_path = input.parse().ok();
        Ok(Self {
            module_ident,
            _comma,
            send_wrapper_path,
        })
    }
}

#[proc_macro_attribute]
pub fn wasm_split(args: TokenStream, input: TokenStream) -> TokenStream {
    let WasmSplitArgs {
        module_ident,
        send_wrapper_path,
        ..
    } = parse_macro_input!(args);
    let item_fn = parse_macro_input!(input as ItemFn);

    let name = &item_fn.sig.ident;

    let preload_name =
        Ident::new(&format!("__preload_{}", item_fn.sig.ident), name.span());

    let unique_identifier = base16::encode_lower(
        &sha2::Sha256::digest(format!("{name} {span:?}", span = name.span()))
            [..16],
    );

    let load_module_ident = format_ident!("__wasm_split_load_{module_ident}");
    let split_loader_ident =
        format_ident!("__wasm_split_loader_{unique_identifier}");
    let impl_import_ident = format_ident!(
        "__wasm_split_00{module_ident}00_import_{unique_identifier}_{name}"
    );
    let impl_export_ident = format_ident!(
        "__wasm_split_00{module_ident}00_export_{unique_identifier}_{name}"
    );

    let mut import_sig = Signature {
        ident: impl_import_ident.clone(),
        asyncness: None,
        ..item_fn.sig.clone()
    };
    let mut export_sig = Signature {
        ident: impl_export_ident.clone(),
        asyncness: None,
        ..item_fn.sig.clone()
    };

    let was_async = item_fn.sig.asyncness.is_some();
    if was_async {
        let ty = match &item_fn.sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        };
        let async_output = parse::<ReturnType>(
            quote! {
                -> std::pin::Pin<Box<dyn std::future::Future<Output = #ty> + Send + Sync>>
            }
            .into(),
        )
        .unwrap();
        export_sig.output = async_output.clone();
        import_sig.output = async_output;
    }

    let wrapper_pub = item_fn.vis;
    let mut wrapper_sig = item_fn.sig;
    wrapper_sig.asyncness = Some(Default::default());
    let mut args = Vec::new();
    for (i, param) in wrapper_sig.inputs.iter_mut().enumerate() {
        match param {
            syn::FnArg::Typed(pat_type) => {
                let param_ident = format_ident!("__wasm_split_arg_{i}");
                args.push(param_ident.clone());
                pat_type.pat = Box::new(syn::Pat::Ident(syn::PatIdent {
                    attrs: vec![],
                    by_ref: None,
                    mutability: None,
                    ident: param_ident,
                    subpat: None,
                }));
            }
            syn::FnArg::Receiver(_) => {
                args.push(format_ident!("self"));
            }
        }
    }

    let attrs = item_fn.attrs;

    let stmts = &item_fn.block.stmts;

    let body = if was_async {
        if let Some(send_wrapper_path) = send_wrapper_path {
            quote! {
                Box::pin(#send_wrapper_path::SendWrapper::new(async move {
                    #(#stmts)*
                }))
            }
        } else {
            quote! {
                Box::pin(async move {
                    #(#stmts)*
                })
            }
        }
    } else {
        quote! { #(#stmts)* }
    };

    let await_result = was_async.then(|| quote! { .await });

    quote! {
        thread_local! {
            static #split_loader_ident: ::leptos::wasm_split_helpers::LazySplitLoader = ::leptos::wasm_split_helpers::LazySplitLoader::new(#load_module_ident);
        }

        #[link(wasm_import_module = "/pkg/__wasm_split.______________________.js")]
        extern "C" {
            #[no_mangle]
            fn #load_module_ident (callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool), data: *const ::std::ffi::c_void) -> ();

            #[allow(improper_ctypes)]
            #[no_mangle]
            #import_sig;
        }

        #[allow(non_snake_case)]
        #(#attrs)*
        #wrapper_pub #wrapper_sig {
            #(#attrs)*
            #[allow(improper_ctypes_definitions)]
            #[allow(non_snake_case)]
            #[no_mangle]
            pub extern "C" #export_sig {
                #body
            }

            ::leptos::wasm_split_helpers::ensure_loaded(&#split_loader_ident).await.unwrap();
            unsafe { #impl_import_ident( #(#args),* ) } #await_result
        }

        #[doc(hidden)]
        #[allow(non_snake_case)]
        #wrapper_pub async fn #preload_name() {
            ::leptos::wasm_split_helpers::ensure_loaded(&#split_loader_ident).await.unwrap();
        }
    }
    .into()
}
