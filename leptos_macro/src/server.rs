use cfg_if::cfg_if;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *,
};

pub fn server_macro_impl(args: proc_macro::TokenStream, s: TokenStream2) -> Result<TokenStream2> {
    let ServerFnName {
        struct_name,
        prefix,
        ..
    } = syn::parse::<ServerFnName>(args)?;
    let prefix = prefix.unwrap_or_else(|| Literal::string(""));

    let body = syn::parse::<ServerFnBody>(s.into())?;
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = body.vis;
    let block = body.block;

    cfg_if! {
        if #[cfg(not(feature = "stable"))] {
            use proc_macro::Span;
            let span = Span::call_site();
            let url = format!("{}/{}", span.source_file().path().to_string_lossy(), fn_name_as_str).replace("/", "-");
        } else {
            let url = fn_name_as_str;
        }
    }

    let fields = body.inputs.iter().map(|f| {
        let typed_arg = match f {
            FnArg::Receiver(_) => panic!("cannot use receiver types in server function macro"),
            FnArg::Typed(t) => t,
        };
        quote! { pub #typed_arg }
    });

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
        FnArg::Typed(t) => Some(&t.pat),
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

        impl ServerFn for #struct_name {
            type Output = #output_ty;

            fn prefix() -> &'static str {
                #prefix
            }

            fn url() -> &'static str {
                #url
            }

            #[cfg(feature = "ssr")]
            fn call_fn(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, ::leptos::ServerFnError>> + Send>> {
                let #struct_name { #(#field_names),* } = self;
                Box::pin(async move { #fn_name( #(#field_names_2),*).await })
            }

            #[cfg(not(feature = "ssr"))]
            fn call_fn_client(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, ::leptos::ServerFnError>>>> {
                let #struct_name { #(#field_names_3),* } = self;
                Box::pin(async move { #fn_name( #(#field_names_4),*).await })
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
            ::leptos::call_server_fn(&url, #struct_name { #(#field_names_5),* }).await
        }
    })
}

pub struct ServerFnName {
    struct_name: Ident,
    _comma: Option<Token![,]>,
    prefix: Option<Literal>,
}

impl Parse for ServerFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;
        let _comma = input.parse()?;
        let prefix = input.parse()?;

        Ok(Self {
            struct_name,
            _comma,
            prefix,
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
