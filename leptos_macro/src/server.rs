use cfg_if::cfg_if;
use leptos_server::Encoding;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *,
};

fn fn_arg_is_cx(f: &syn::FnArg) -> bool {
    if let FnArg::Typed(t) = f {
        if let Type::Path(path) = &*t.ty {
            path.path
                .segments
                .iter()
                .any(|segment| segment.ident == "Scope")
        } else {
            false
        }
    } else {
        false
    }
}

pub fn server_macro_impl(args: proc_macro::TokenStream, s: TokenStream2) -> Result<TokenStream2> {
    let ServerFnName {
        struct_name,
        prefix,
        encoding,
        ..
    } = syn::parse::<ServerFnName>(args)?;
    let prefix = prefix.unwrap_or_else(|| Literal::string(""));
    let encoding = match encoding {
        Encoding::Cbor => quote! { ::leptos::Encoding::Cbor },
        Encoding::Url => quote! { ::leptos::Encoding::Url },
    };

    let body = syn::parse::<ServerFnBody>(s.into())?;
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = body.vis;
    let block = body.block;

    cfg_if! {
        if #[cfg(not(feature = "stable"))] {
            use proc_macro::Span;
            let span = Span::call_site();
            #[cfg(not(target_os = "windows"))]
            let url = format!("{}/{}", span.source_file().path().to_string_lossy(), fn_name_as_str).replace('/', "-");
            #[cfg(target_os = "windows")]
            let url = format!("{}/{}", span.source_file().path().to_string_lossy(), fn_name_as_str).replace("\\", "-");
        } else {
            let url = fn_name_as_str;
        }
    }

    let fields = body.inputs.iter().filter(|f| !fn_arg_is_cx(f)).map(|f| {
        let typed_arg = match f {
            FnArg::Receiver(_) => panic!("cannot use receiver types in server function macro"),
            FnArg::Typed(t) => t,
        };
        quote! { pub #typed_arg }
    });

    let cx_arg = body
        .inputs
        .iter()
        .next()
        .and_then(|f| if fn_arg_is_cx(f) { Some(f) } else { None });
    let cx_assign_statement = if let Some(FnArg::Typed(arg)) = cx_arg {
        if let Pat::Ident(id) = &*arg.pat {
            quote! {
                let #id = cx;
            }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };
    let cx_fn_arg = if cx_arg.is_some() {
        quote! { cx, }
    } else {
        quote! {}
    };

    let fn_args = body.inputs.iter().map(|f| {
        let typed_arg = match f {
            FnArg::Receiver(_) => panic!("cannot use receiver types in server function macro"),
            FnArg::Typed(t) => t,
        };
        quote! { #typed_arg }
    });
    let fn_args_2 = fn_args.clone();

    let field_names = body.inputs.iter().filter_map(|f| match f {
        FnArg::Receiver(_) => todo!(),
        FnArg::Typed(t) => {
            if fn_arg_is_cx(f) {
                None
            } else {
                Some(&t.pat)
            }
        }
    });

    let field_names_2 = field_names.clone();
    let field_names_3 = field_names.clone();
    let field_names_4 = field_names.clone();
    let field_names_5 = field_names.clone();

    let output_arrow = body.output_arrow;
    let return_ty = body.return_ty;

    let output_ty = if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if let PathArguments::AngleBracketed(args) = &pat.path.segments[0].arguments {
                &args.args[0]
            } else {
                panic!("server functions should return Result<T, ServerFnError>");
            }
        } else {
            panic!("server functions should return Result<T, ServerFnError>");
        }
    } else {
        panic!("server functions should return Result<T, ServerFnError>");
    };

    Ok(quote::quote! {
        #[derive(Clone, ::serde::Serialize, ::serde::Deserialize)]
        pub struct #struct_name {
            #(#fields),*
        }

        impl leptos::ServerFn for #struct_name {
            type Output = #output_ty;

            fn prefix() -> &'static str {
                #prefix
            }

            fn url() -> &'static str {
                #url
            }

            fn encoding() -> ::leptos::Encoding {
                #encoding
            }

            #[cfg(any(feature = "ssr", doc))]
            fn call_fn(self, cx: ::leptos::Scope) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, ::leptos::ServerFnError>>>> {
                let #struct_name { #(#field_names),* } = self;
                #cx_assign_statement;
                Box::pin(async move { #fn_name( #cx_fn_arg #(#field_names_2),*).await })
            }

            #[cfg(any(not(feature = "ssr"), doc))]
            fn call_fn_client(self, cx: ::leptos::Scope) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, ::leptos::ServerFnError>>>> {
                let #struct_name { #(#field_names_3),* } = self;
                Box::pin(async move { #fn_name( #cx_fn_arg #(#field_names_4),*).await })
            }
        }

        #[cfg(feature = "ssr")]
        #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
            #block
        }
        #[cfg(not(feature = "ssr"))]
        #vis async fn #fn_name(#(#fn_args_2),*) #output_arrow #return_ty {
            let prefix = #struct_name::prefix().to_string();
            let url = prefix + "/" + #struct_name::url();
            ::leptos::call_server_fn(&url, #struct_name { #(#field_names_5),* }, #encoding).await
        }
    })
}

pub struct ServerFnName {
    struct_name: Ident,
    _comma: Option<Token![,]>,
    prefix: Option<Literal>,
    _comma2: Option<Token![,]>,
    encoding: Encoding,
}

impl Parse for ServerFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;
        let _comma = input.parse()?;
        let prefix = input.parse()?;
        let _comma2 = input.parse()?;
        let encoding = input.parse().unwrap_or(Encoding::Url);

        Ok(Self {
            struct_name,
            _comma,
            prefix,
            _comma2,
            encoding,
        })
    }
}

pub struct ServerFnBody {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub async_token: Token![async],
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    pub output_arrow: Token![->],
    pub return_ty: syn::Type,
    pub block: Box<Block>,
}

/// The custom rusty variant of parsing rsx!
impl Parse for ServerFnBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;

        let async_token = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let paren_token = syn::parenthesized!(content in input);

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output_arrow = input.parse()?;
        let return_ty = input.parse()?;

        let block = input.parse()?;

        Ok(Self {
            vis,
            async_token,
            fn_token,
            ident,
            generics,
            paren_token,
            inputs,
            output_arrow,
            return_ty,
            block,
            attrs,
        })
    }
}
