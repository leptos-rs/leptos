#![cfg_attr(feature = "nightly", feature(proc_macro_span))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Implementation of the `server_fn` macro.
//!
//! This crate contains the implementation of the `server_fn` macro. [`server_macro_impl`] can be used to implement custom versions of the macro for different frameworks that allow users to pass a custom context from the server to the server function.

use convert_case::{Case, Converter};
use proc_macro2::{Literal, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};

/// The implementation of the `server_fn` macro.
/// To allow the macro to accept a custom context from the server, pass a custom server context to this function.
/// **The Context comes from the server.** Optionally, the first argument of a server function
/// can be a custom context. This context can be used to inject dependencies like the HTTP request
/// or response or other server-only dependencies, but it does *not* have access to state that exists in the client.
///
/// The paths passed into this function are used in the generated code, so they must be in scope when the macro is called.
///
/// ```ignore
/// #[proc_macro_attribute]
/// pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
///     match server_macro_impl(
///         args.into(),
///         s.into(),
///         Some(syn::parse_quote!(my_crate::exports::server_fn)),
///     ) {
///         Err(e) => e.to_compile_error().into(),
///         Ok(s) => s.to_token_stream().into(),
///     }
/// }
/// ```
pub fn server_macro_impl(
    args: TokenStream2,
    body: TokenStream2,
    server_fn_path: Option<Path>,
    default_path: &str,
) -> Result<TokenStream2> {
    let mut body = syn::parse::<ServerFnBody>(body.into())?;

    // extract all #[middleware] attributes, removing them from signature of dummy
    let mut middlewares: Vec<Middleware> = vec![];
    body.attrs.retain(|attr| {
        if attr.meta.path().is_ident("middleware") {
            if let Ok(middleware) = attr.parse_args() {
                middlewares.push(middleware);
                false
            } else {
                true
            }
        } else {
            true
        }
    });

    let dummy = body.to_dummy_output();
    let dummy_name = body.to_dummy_ident();
    let args = syn::parse::<ServerFnArgs>(args.into())?;

    // default values for args
    let ServerFnArgs {
        struct_name,
        prefix,
        input,
        output,
        fn_path,
    } = args;
    let prefix = prefix.unwrap_or_else(|| Literal::string(default_path));
    let fn_path = fn_path.unwrap_or_else(|| Literal::string(""));
    let input = input.unwrap_or_else(|| syn::parse_quote!(PostUrl));
    let input_is_rkyv = input == "Rkyv";
    let input_is_multipart = input == "MultipartFormData";
    let input = codec_ident(server_fn_path.as_ref(), input);
    let output = output.unwrap_or_else(|| syn::parse_quote!(Json));
    let output = codec_ident(server_fn_path.as_ref(), output);
    // default to PascalCase version of function name if no struct name given
    let struct_name = struct_name.unwrap_or_else(|| {
        let upper_camel_case_name = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::UpperCamel)
            .convert(body.ident.to_string());
        Ident::new(&upper_camel_case_name, body.ident.span())
    });

    // build struct for type
    let mut body = body;
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = body.vis;
    let block = body.block;
    let attrs = body.attrs;

    let fields = body
        .inputs
        .iter_mut()
        .map(|f| {
            let typed_arg = match f {
                FnArg::Receiver(_) => {
                    return Err(syn::Error::new(
                        f.span(),
                        "cannot use receiver types in server function macro",
                    ))
                }
                FnArg::Typed(t) => t,
            };
            // allow #[server(default)] on fields — TODO is this documented?
            let mut default = false;
            let mut other_attrs = Vec::new();
            for attr in typed_arg.attrs.iter() {
                if !attr.path().is_ident("server") {
                    other_attrs.push(attr.clone());
                    continue;
                }
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("default") && meta.input.is_empty() {
                        default = true;
                        Ok(())
                    } else {
                        Err(meta.error(
                            "Unrecognized #[server] attribute, expected \
                             #[server(default)]",
                        ))
                    }
                })?;
            }
            typed_arg.attrs = other_attrs;
            if default {
                Ok(quote! { #[serde(default)] pub #typed_arg })
            } else {
                Ok(quote! { pub #typed_arg })
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let fn_args = body
        .inputs
        .iter()
        .filter_map(|f| match f {
            FnArg::Receiver(_) => None,
            FnArg::Typed(t) => Some(t),
        })
        .collect::<Vec<_>>();

    let field_names = body
        .inputs
        .iter()
        .filter_map(|f| match f {
            FnArg::Receiver(_) => None,
            FnArg::Typed(t) => Some(&t.pat),
        })
        .collect::<Vec<_>>();

    // if there's exactly one field, impl From<T> for the struct
    let first_field = body.inputs.iter().find_map(|f| match f {
        FnArg::Receiver(_) => None,
        FnArg::Typed(t) => Some((&t.pat, &t.ty)),
    });
    let from_impl =
        (body.inputs.len() == 1 && first_field.is_some()).then(|| {
            let field = first_field.unwrap();
            let (name, ty) = field;
            quote! {
                impl From<#struct_name> for #ty {
                    fn from(value: #struct_name) -> Self {
                        let #struct_name { #name } = value;
                        #name
                    }
                }

                impl From<#ty> for #struct_name {
                    fn from(#name: #ty) -> Self {
                        #struct_name { #name }
                    }
                }
            }
        });

    // check output type
    let output_arrow = body.output_arrow;
    let return_ty = body.return_ty;

    let output_ty = output_type(&return_ty)?;
    let error_ty = err_type(&return_ty)?;
    let error_ty =
        error_ty.map(ToTokens::to_token_stream).unwrap_or_else(|| {
            quote! {
                #server_fn_path::error::NoCustomError
            }
        });

    // build server fn path
    let serde_path = server_fn_path.as_ref().map(|path| {
        let path = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let path = path.join("::");
        format!("{path}::serde")
    });
    let server_fn_path = server_fn_path
        .map(|path| quote!(#path))
        .unwrap_or_else(|| quote! { server_fn });

    let key_env_var = match option_env!("SERVER_FN_OVERRIDE_KEY") {
        Some(_) => "SERVER_FN_OVERRIDE_KEY",
        None => "CARGO_MANIFEST_DIR",
    };

    let link_to_server_fn = format!(
        "Serialized arguments for the [`{fn_name_as_str}`] server \
         function.\n\n"
    );
    let args_docs = quote! {
        #[doc = #link_to_server_fn]
    };

    // pass through docs
    let docs = body
        .docs
        .iter()
        .map(|(doc, span)| quote_spanned!(*span=> #[doc = #doc]))
        .collect::<TokenStream2>();

    // auto-registration with inventory
    let inventory = if cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::inventory::submit! {{
                use #server_fn_path::{ServerFn, codec::Encoding};
                #server_fn_path::ServerFnTraitObj::new(
                    #struct_name::PATH,
                    <#struct_name as ServerFn>::InputEncoding::METHOD,
                    |req| {
                        Box::pin(#struct_name::run_on_server(req))
                    },
                    #struct_name::middlewares
                )
            }}
        }
    } else {
        quote! {}
    };

    // run_body in the trait implementation
    let run_body = if cfg!(feature = "ssr") {
        quote! {
            async fn run_body(self) -> #return_ty {
                let #struct_name { #(#field_names),* } = self;
                #dummy_name(#(#field_names),*).await
            }
        }
    } else {
        quote! {
            #[allow(unused_variables)]
            async fn run_body(self) -> #return_ty {
                unreachable!()
            }
        }
    };

    // the actual function definition
    let func = if cfg!(feature = "ssr") {
        quote! {
            #docs
            #(#attrs)*
            #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
                #block
            }
        }
    } else {
        quote! {
            #docs
            #(#attrs)*
            #[allow(unused_variables)]
            #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
                use #server_fn_path::ServerFn;
                let data = #struct_name { #(#field_names),* };
                data.run_on_client().await
            }
        }
    };

    // TODO rkyv derives
    let derives = if input_is_multipart {
        quote! {}
    } else if input_is_rkyv {
        todo!("implement derives for Rkyv")
    } else {
        quote! {
            Clone, #server_fn_path::serde::Serialize, #server_fn_path::serde::Deserialize
        }
    };
    let serde_path = (!input_is_multipart && !input_is_rkyv).then(|| {
        quote! {
            #[serde(crate = #serde_path)]
        }
    });

    // TODO reqwest
    let client = quote! {
        #server_fn_path::client::browser::BrowserClient
    };

    let req = if !cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::request::BrowserMockReq
        }
    } else if cfg!(feature = "axum") {
        quote! {
            #server_fn_path::axum_export::http::Request<#server_fn_path::axum_export::body::Body>
        }
    } else if cfg!(feature = "actix") {
        quote! {
            #server_fn_path::actix_export::HttpRequest
        }
    } else {
        return Err(syn::Error::new(
            Span::call_site(),
            "If the `ssr` feature is enabled, either the `actix` or `axum` \
             features should also be enabled.",
        ));
    };
    let res = if !cfg!(feature = "ssr") {
        quote! {
            #server_fn_path::response::BrowserMockRes
        }
    } else if cfg!(feature = "axum") {
        quote! {
            #server_fn_path::axum_export::http::Response<#server_fn_path::axum_export::body::Body>
        }
    } else if cfg!(feature = "actix") {
        quote! {
            #server_fn_path::actix_export::HttpResponse
        }
    } else {
        return Err(syn::Error::new(
            Span::call_site(),
            "If the `ssr` feature is enabled, either the `actix` or `axum` \
             features should also be enabled.",
        ));
    };

    // generate path
    let path = quote! {
        if #fn_path.is_empty() {
            #server_fn_path::const_format::concatcp!(
                #prefix,
                "/",
                #fn_name_as_str,
                #server_fn_path::xxhash_rust::const_xxh64::xxh64(
                    concat!(env!(#key_env_var), ":", file!(), ":", line!(), ":", column!()).as_bytes(),
                    0
                )
            )
        } else {
            #server_fn_path::const_format::concatcp!(
                #prefix,
                #fn_path
            )
        }
    };

    // only emit the dummy (unmodified server-only body) for the server build
    let dummy = cfg!(feature = "ssr").then_some(dummy);
    let middlewares = if cfg!(feature = "ssr") {
        quote! {
            vec![
                #(
                    std::sync::Arc::new(#middlewares),
                ),*
            ]
        }
    } else {
        quote! { vec![] }
    };

    Ok(quote::quote! {
        #args_docs
        #docs
        #[derive(Debug, #derives)]
        #serde_path
        pub struct #struct_name {
            #(#fields),*
        }

        #from_impl

        impl #server_fn_path::ServerFn for #struct_name {
            // TODO prefix
            const PATH: &'static str = #path;

            type Client = #client;
            type ServerRequest = #req;
            type ServerResponse = #res;
            type Output = #output_ty;
            type InputEncoding = #input;
            type OutputEncoding = #output;
            type Error = #error_ty;

            fn middlewares() -> Vec<std::sync::Arc<dyn #server_fn_path::middleware::Layer<#req, #res>>> {
                #middlewares
            }

            #run_body
        }

        #inventory

        #func

        #dummy
    })
}

#[derive(Debug)]
struct Middleware {
    expr: syn::Expr,
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let expr = &self.expr;
        tokens.extend(quote::quote! {
            #expr
        });
    }
}

impl Parse for Middleware {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg: syn::Expr = input.parse()?;
        Ok(Middleware { expr: arg })
    }
}

fn output_type(return_ty: &Type) -> Result<&GenericArgument> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if pat.path.segments.is_empty() {
                panic!("{:#?}", pat.path);
            } else if let PathArguments::AngleBracketed(args) =
                &pat.path.segments[0].arguments
            {
                return Ok(&args.args[0]);
            }
        }
    };

    Err(syn::Error::new(
        return_ty.span(),
        "server functions should return Result<T, ServerFnError> or Result<T, \
         ServerFnError<E>>",
    ))
}

fn err_type(return_ty: &Type) -> Result<Option<&GenericArgument>> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if let PathArguments::AngleBracketed(args) =
                &pat.path.segments[0].arguments
            {
                // Result<T>
                if args.args.len() == 1 {
                    return Ok(None);
                }
                // Result<T, _>
                else if let GenericArgument::Type(Type::Path(pat)) =
                    &args.args[1]
                {
                    if pat.path.segments[0].ident == "ServerFnError" {
                        let args = &pat.path.segments[0].arguments;
                        match args {
                            // Result<T, ServerFnError>
                            PathArguments::None => return Ok(None),
                            // Result<T, ServerFnError<E>>
                            PathArguments::AngleBracketed(args) => {
                                if args.args.len() == 1 {
                                    return Ok(Some(&args.args[0]));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    };

    Err(syn::Error::new(
        return_ty.span(),
        "server functions should return Result<T, ServerFnError> or Result<T, \
         ServerFnError<E>>",
    ))
}

#[derive(Debug)]
struct ServerFnArgs {
    struct_name: Option<Ident>,
    prefix: Option<Literal>,
    input: Option<Ident>,
    output: Option<Ident>,
    fn_path: Option<Literal>,
}

impl Parse for ServerFnArgs {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        // legacy 4-part arguments
        let mut struct_name: Option<Ident> = None;
        let mut prefix: Option<Literal> = None;
        let mut encoding: Option<Literal> = None;
        let mut fn_path: Option<Literal> = None;

        // new arguments: can only be keyed by name
        let mut input: Option<Ident> = None;
        let mut output: Option<Ident> = None;

        let mut use_key_and_value = false;
        let mut arg_pos = 0;

        while !stream.is_empty() {
            arg_pos += 1;
            let lookahead = stream.lookahead1();
            if lookahead.peek(Ident) {
                let key_or_value: Ident = stream.parse()?;

                let lookahead = stream.lookahead1();
                if lookahead.peek(Token![=]) {
                    stream.parse::<Token![=]>()?;
                    let key = key_or_value;
                    use_key_and_value = true;
                    if key == "name" {
                        if struct_name.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `name`",
                            ));
                        }
                        struct_name = Some(stream.parse()?);
                    } else if key == "prefix" {
                        if prefix.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `prefix`",
                            ));
                        }
                        prefix = Some(stream.parse()?);
                    } else if key == "encoding" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `encoding`",
                            ));
                        }
                        encoding = Some(stream.parse()?);
                    } else if key == "endpoint" {
                        if fn_path.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `endpoint`",
                            ));
                        }
                        fn_path = Some(stream.parse()?);
                    } else if key == "input" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `input` should not both be \
                                 specified",
                            ));
                        } else if input.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `input`",
                            ));
                        }
                        input = Some(stream.parse()?);
                    } else if key == "output" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `output` should not both be \
                                 specified",
                            ));
                        } else if output.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `output`",
                            ));
                        }
                        output = Some(stream.parse()?);
                    } else {
                        return Err(lookahead.error());
                    }
                } else {
                    let value = key_or_value;
                    if use_key_and_value {
                        return Err(syn::Error::new(
                            value.span(),
                            "positional argument follows keyword argument",
                        ));
                    }
                    if arg_pos == 1 {
                        struct_name = Some(value)
                    } else {
                        return Err(syn::Error::new(
                            value.span(),
                            "expected string literal",
                        ));
                    }
                }
            } else if lookahead.peek(LitStr) {
                let value: Literal = stream.parse()?;
                if use_key_and_value {
                    return Err(syn::Error::new(
                        value.span(),
                        "If you use keyword arguments (e.g., `name` = \
                         Something), then you can no longer use arguments \
                         without a keyword.",
                    ));
                }
                match arg_pos {
                    1 => return Err(lookahead.error()),
                    2 => prefix = Some(value),
                    3 => encoding = Some(value),
                    4 => fn_path = Some(value),
                    _ => {
                        return Err(syn::Error::new(
                            value.span(),
                            "unexpected extra argument",
                        ))
                    }
                }
            } else {
                return Err(lookahead.error());
            }

            if !stream.is_empty() {
                stream.parse::<Token![,]>()?;
            }
        }

        // parse legacy encoding into input/output
        if let Some(encoding) = encoding {
            match encoding.to_string().to_lowercase().as_str() {
                "\"url\"" => {
                    input = syn::parse_quote!(PostUrl);
                    output = syn::parse_quote!(Json);
                }
                "\"cbor\"" => {
                    input = syn::parse_quote!(Cbor);
                    output = syn::parse_quote!(Cbor);
                }
                "\"getcbor\"" => {
                    input = syn::parse_quote!(GetUrl);
                    output = syn::parse_quote!(Cbor);
                }
                "\"getjson\"" => {
                    input = syn::parse_quote!(GetUrl);
                    output = syn::parse_quote!(Json);
                }
                _ => {
                    return Err(syn::Error::new(
                        encoding.span(),
                        "Encoding not found.",
                    ))
                }
            }
        }

        Ok(Self {
            _attrs,
            struct_name,
            prefix,
            input,
            output,
            fn_path,
        })
    }
}

#[derive(Debug)]
struct ServerFnBody {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub async_token: Token![async],
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub _paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    pub output_arrow: Token![->],
    pub return_ty: syn::Type,
    pub block: TokenStream2,
    pub docs: Vec<(String, Span)>,
}

impl Parse for ServerFnBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;

        let async_token = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let _paren_token = syn::parenthesized!(content in input);

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output_arrow = input.parse()?;
        let return_ty = input.parse()?;

        let block = input.parse()?;

        let docs = attrs
            .iter()
            .filter_map(|attr| {
                let Meta::NameValue(attr) = &attr.meta else {
                    return None;
                };
                if !attr.path.is_ident("doc") {
                    return None;
                }

                let value = match &attr.value {
                    syn::Expr::Lit(lit) => match &lit.lit {
                        syn::Lit::Str(s) => Some(s.value()),
                        _ => return None,
                    },
                    _ => return None,
                };

                Some((value.unwrap_or_default(), attr.path.span()))
            })
            .collect();
        attrs.retain(|attr| {
            let Meta::NameValue(attr) = &attr.meta else {
                return true;
            };
            !attr.path.is_ident("doc")
        });

        Ok(Self {
            vis,
            async_token,
            fn_token,
            ident,
            generics,
            _paren_token,
            inputs,
            output_arrow,
            return_ty,
            block,
            attrs,
            docs,
        })
    }
}

impl ServerFnBody {
    fn to_dummy_ident(&self) -> Ident {
        Ident::new(&format!("__{}", self.ident), self.ident.span())
    }

    fn to_dummy_output(&self) -> TokenStream2 {
        let ident = self.to_dummy_ident();
        let Self {
            attrs,
            vis,
            async_token,
            fn_token,
            generics,
            inputs,
            output_arrow,
            return_ty,
            block,
            ..
        } = &self;
        quote! {
            #[doc(hidden)]
            #(#attrs)*
            #vis #async_token #fn_token #ident #generics ( #inputs ) #output_arrow #return_ty
            #block
        }
    }
}

/// Returns either the path of the codec (if it's a builtin) or the
/// original ident.
fn codec_ident(server_fn_path: Option<&Path>, ident: Ident) -> TokenStream2 {
    if let Some(server_fn_path) = server_fn_path {
        let str = ident.to_string();
        if [
            "GetUrl",
            "PostUrl",
            "Cbor",
            "Json",
            "Rkyv",
            "Streaming",
            "StreamingText",
            "MultipartFormData",
        ]
        .contains(&str.as_str())
        {
            return quote! {
                #server_fn_path::codec::#ident
            };
        }
    }

    ident.into_token_stream()
}
