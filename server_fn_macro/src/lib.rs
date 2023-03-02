#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Implementation of the server_fn macro.
//!
//! This crate contains the implementation of the server_fn macro. [server_macro_impl] can be used to implement custom versions of the macro for different frameworks that allow users to pass a custom context from the server to the server function.

use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};

/// Discribes the custom context from the server that passed to the server function. Optionally, the first argument of a server function
/// can be a custom context of this type. This context can be used to access the server's state within the server function.
pub struct ServerContext {
    /// The type of the context.
    pub ty: Ident,
    /// The path to the context type. Used to reference the context type in the generated code.
    pub path: Path,
}

fn fn_arg_is_cx(f: &syn::FnArg, server_context: &ServerContext) -> bool {
    if let FnArg::Typed(t) = f {
        if let Type::Path(path) = &*t.ty {
            path.path
                .segments
                .iter()
                .any(|segment| segment.ident == server_context.ty)
        } else {
            false
        }
    } else {
        false
    }
}

/// The implementation of the server_fn macro.
/// To allow the macro to accept a custom context from the server, pass a custom server context to this function.
/// **The Context comes from the server.** Optionally, the first argument of a server function
/// can be a custom context. This context can be used to inject dependencies like the HTTP request
/// or response or other server-only dependencies, but it does *not* have access to state that exists in the client.
///
/// The paths passed into this function are used in the generated code, so they must be in scope when the macro is called.
///
/// # Macro Crate
/// ```rust, ignore
/// #[proc_macro_attribute]
/// pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
///     let server_context = Some(ServerContext {
///         ty: syn::parse_quote!(MyContext),
///         path: syn::parse_quote!(my_crate::prelude::MyContext),
///     });
///     match server_macro_impl(
///         args.into(),
///         s.into(),
///         Some(server_context),
///         Some(syn::parse_quote!(my_crate::exports::server_fn)),
///     ) {
///         Err(e) => e.to_compile_error().into(),
///         Ok(s) => s.to_token_stream().into(),
///     }
/// }
/// ```
///
/// # Main Crate
/// ```rust, ignore
/// pub use macro_crate::server;
///
/// // collect the server functions into a map
/// #[cfg(any(feature = "ssr", doc))]
/// lazy_static::lazy_static! {
///     static ref REGISTERED_SERVER_FUNCTIONS: Arc<RwLock<HashMap<&'static str, &'static MyServerFnTraitObj>>> = {
///         let mut map = HashMap::new();
///         for server_fn in inventory::iter::<MyServerFnTraitObj> {
///             map.insert(server_fn.0.url(), server_fn);
///         }
///         Arc::new(RwLock::new(map))
///     };
/// }
///
/// // collect all of the server functions into an iterator
/// #[cfg(any(feature = "ssr", doc))]
/// inventory::collect!(MyServerFnTraitObj);
///
/// // a server function wrapper that your framework can use to call the server function
/// #[cfg(any(feature = "ssr", doc))]
/// pub struct MyServerFnTraitObj(ServerFnTraitObj<MyContext>);
///
/// #[cfg(any(feature = "ssr", doc))]
/// impl MyServerFnTraitObj {
///     // This *MUST* be called `from_generic_server_fn` and be const for the macro to work.
///     pub const fn from_generic_server_fn(f: ServerFnTraitObj<MyContext>) -> Self {
///         Self(f)
///     }
/// }
/// ```

pub fn server_macro_impl(
    args: TokenStream2,
    body: TokenStream2,
    trait_obj_wrapper: Type,
    server_context: Option<ServerContext>,
    server_fn_path: Option<Path>,
) -> Result<TokenStream2> {
    let ServerFnName {
        struct_name,
        prefix,
        encoding,
        ..
    } = syn::parse2::<ServerFnName>(args)?;
    let prefix = prefix.unwrap_or_else(|| Literal::string(""));
    let encoding = quote!(#server_fn_path::#encoding);

    let body = syn::parse::<ServerFnBody>(body.into())?;
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = body.vis;
    let block = body.block;

    let fields: Result<Vec<_>> = body
        .inputs
        .iter()
        .filter(|f| {
            if let Some(ctx) = &server_context {
                !fn_arg_is_cx(f, ctx)
            } else {
                true
            }
        })
        .map(|f| match f {
            FnArg::Receiver(_) => Err(Error::new(
                f.span(),
                "cannot use receiver types in server function macro",
            )),
            FnArg::Typed(t) => Ok(quote! { pub #t }),
        })
        .collect();
    let fields = fields?;

    let cx_arg = body.inputs.iter().next().and_then(|f| {
        server_context
            .as_ref()
            .and_then(|ctx| fn_arg_is_cx(f, ctx).then_some(f))
    });
    let cx_assign_statement = if let Some(FnArg::Typed(arg)) = cx_arg {
        if let Pat::Ident(id) = &*arg.pat {
            quote! {
                #[allow(unused)]
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

    let fn_args: Result<Vec<_>> = body
        .inputs
        .iter()
        .map(|f| {
            let typed_arg = match f {
                FnArg::Receiver(_) => {
                    return Err(Error::new(
                        f.span(),
                        "cannot use receiver types in server function macro",
                    ));
                }
                FnArg::Typed(t) => t,
            };
            let is_cx = if let Some(ctx) = &server_context {
                !fn_arg_is_cx(f, ctx)
            } else {
                true
            };
            Ok(if is_cx {
                quote! {
                    #[allow(unused)]
                    #typed_arg
                }
            } else {
                quote! { #typed_arg }
            })
        })
        .collect();
    let fn_args = fn_args?;
    let fn_args_2 = fn_args.clone();

    let field_names = body.inputs.iter().filter_map(|f| match f {
        FnArg::Receiver(_) => todo!(),
        FnArg::Typed(t) => {
            if let Some(ctx) = &server_context {
                if fn_arg_is_cx(f, ctx) {
                    None
                } else {
                    Some(&t.pat)
                }
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

    let output_ty = 'output_ty: {
        if let syn::Type::Path(pat) = &return_ty {
            if pat.path.segments[0].ident == "Result" {
                if let PathArguments::AngleBracketed(args) =
                    &pat.path.segments[0].arguments
                {
                    break 'output_ty &args.args[0];
                }
            }
        }

        return Err(Error::new(
            return_ty.span(),
            "server functions should return Result<T, ServerFnError>",
        ));
    };

    let server_ctx_path = if let Some(ctx) = &server_context {
        let path = &ctx.path;
        quote!(#path)
    } else {
        quote!(())
    };

    let server_fn_path = server_fn_path
        .map(|path| quote!(#path))
        .unwrap_or_else(|| quote! { server_fn });

    Ok(quote::quote! {
        #[derive(Clone, Debug, #server_fn_path::serde::Serialize, #server_fn_path::serde::Deserialize)]
        pub struct #struct_name {
            #(#fields),*
        }

        impl #struct_name {
            const URL: &str = #server_fn_path::const_format::concatcp!(#fn_name_as_str, #server_fn_path::xxhash_rust::const_xxh64::xxh64(concat!(env!("CARGO_MANIFEST_DIR"), ":", file!(), ":", line!(), ":", column!()).as_bytes(), 0));
            const PREFIX: &str = #prefix;
            const ENCODING: #server_fn_path::Encoding = #encoding;
        }

        #[cfg(feature = "ssr")]
        #server_fn_path::inventory::submit! {
            #trait_obj_wrapper::from_generic_server_fn(#server_fn_path::ServerFnTraitObj::new(
                #struct_name::PREFIX,
                #struct_name::URL,
                #struct_name::ENCODING,
                <#struct_name as #server_fn_path::ServerFn<#server_ctx_path>>::call_from_bytes,
            ))
        }

        impl #server_fn_path::ServerFn<#server_ctx_path> for #struct_name {
            type Output = #output_ty;

            fn prefix() -> &'static str {
                Self::PREFIX
            }

            fn url() -> &'static str {
                Self::URL
            }

            fn encoding() -> #server_fn_path::Encoding {
                Self::ENCODING
            }

            #[cfg(feature = "ssr")]
            fn call_fn(self, cx: #server_ctx_path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, #server_fn_path::ServerFnError>>>> {
                let #struct_name { #(#field_names),* } = self;
                #cx_assign_statement;
                Box::pin(async move { #fn_name( #cx_fn_arg #(#field_names_2),*).await })
            }

            #[cfg(not(feature = "ssr"))]
            fn call_fn_client(self, cx: #server_ctx_path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, #server_fn_path::ServerFnError>>>> {
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
            let prefix = #struct_name::PREFIX.to_string();
            let url = prefix + "/" + #struct_name::URL;
            #server_fn_path::call_server_fn(&url, #struct_name { #(#field_names_5),* }, #encoding).await
        }
    })
}

struct ServerFnName {
    struct_name: Ident,
    _comma: Option<Token![,]>,
    prefix: Option<Literal>,
    _comma2: Option<Token![,]>,
    encoding: Path,
}

impl Parse for ServerFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;
        let _comma = input.parse()?;
        let prefix = input.parse()?;
        let _comma2 = input.parse()?;
        let encoding = input
            .parse::<Literal>()
            .map(|lit| lit.to_string())
            .unwrap_or("\"Url\"".to_string());
        let encoding = match &*encoding {
            "\"Url\"" => syn::parse_quote!(Encoding::Url),
            "\"Cbor\"" => syn::parse_quote!(Encoding::Cbor),
            _ => {
                return Err(syn::Error::new(
                    encoding.span(),
                    "Encoding Not Found",
                ))
            }
        };

        Ok(Self {
            struct_name,
            _comma,
            prefix,
            _comma2,
            encoding,
        })
    }
}

#[allow(unused)]
struct ServerFnBody {
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
