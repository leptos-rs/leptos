use convert_case::{Case, Converter};
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{ToTokens, __private::TokenStream as TokenStream2};
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Ident, ItemFn, LitStr, Token,
};

pub fn server_impl(args: TokenStream, s: TokenStream) -> TokenStream {
    let function: syn::ItemFn = match syn::parse(s.clone()) {
        Ok(f) => f,
        // Returning the original input stream in the case of a parsing
        // error helps IDEs and rust-analyzer with auto-completion.
        Err(_) => return s,
    };
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    // TODO apply middleware: https://github.com/leptos-rs/leptos/issues/1461
    let mapped_body = quote::quote! {
        #(#attrs)*
        #vis #sig {
            #block
        }
    };

    let mut args: ServerFnArgs = match syn::parse(args) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };
    args.docs = attrs
        .iter()
        .filter(|attr| attr.meta.path().is_ident("doc"))
        .cloned()
        .collect();
    // default to PascalCase version of function name if no struct name given
    if args.struct_name.is_none() {
        let upper_camel_case_name = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::UpperCamel)
            .convert(sig.ident.to_string());
        args.struct_name =
            Some(Ident::new(&upper_camel_case_name, sig.ident.span()));
    }
    // default to "/api" if no prefix given
    if args.prefix.is_none() {
        args.prefix = Some(Literal::string("/api"));
    }
    // default to "Url" if no encoding given
    if args.encoding.is_none() {
        args.encoding = Some(Literal::string("Url"));
    }

    match server_fn_macro::server_macro_impl(
        quote::quote!(#args),
        mapped_body,
        syn::parse_quote!(::leptos::leptos_server::ServerFnTraitObj),
        None,
        Some(syn::parse_quote!(::leptos::server_fn)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

struct ServerFnArgs {
    docs: Vec<Attribute>,
    struct_name: Option<Ident>,
    prefix: Option<Literal>,
    encoding: Option<Literal>,
    fn_path: Option<Literal>,
}

impl ToTokens for ServerFnArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let struct_name =
            self.struct_name.as_ref().map(|s| quote::quote! { #s, });
        let prefix = self.prefix.as_ref().map(|p| quote::quote! { #p, });
        let encoding = self.encoding.as_ref().map(|e| quote::quote! { #e, });
        let fn_path = self.fn_path.as_ref().map(|f| quote::quote! { #f });
        let docs = &self.docs;
        tokens.extend(quote::quote! {
            #(#docs)*
            #struct_name
            #prefix
            #encoding
            #fn_path
        })
    }
}

impl Parse for ServerFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut struct_name: Option<Ident> = None;
        let mut prefix: Option<Literal> = None;
        let mut encoding: Option<Literal> = None;
        let mut fn_path: Option<Literal> = None;

        let mut use_key_and_value = false;
        let mut arg_pos = 0;

        while !input.is_empty() {
            arg_pos += 1;
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident) {
                let key_or_value: Ident = input.parse()?;

                let lookahead = input.lookahead1();
                if lookahead.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    let key = key_or_value;
                    use_key_and_value = true;
                    if key == "name" {
                        if struct_name.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: name",
                            ));
                        }
                        struct_name = Some(input.parse()?);
                    } else if key == "prefix" {
                        if prefix.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: prefix",
                            ));
                        }
                        prefix = Some(input.parse()?);
                    } else if key == "encoding" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: encoding",
                            ));
                        }
                        encoding = Some(input.parse()?);
                    } else if key == "endpoint" {
                        if fn_path.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: endpoint",
                            ));
                        }
                        fn_path = Some(input.parse()?);
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
                let value: Literal = input.parse()?;
                if use_key_and_value {
                    return Err(syn::Error::new(
                        value.span(),
                        "positional argument follows keyword argument",
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

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            docs: vec![],
            struct_name,
            prefix,
            encoding,
            fn_path,
        })
    }
}
