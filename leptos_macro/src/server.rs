use convert_case::{Case, Converter};
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{ToTokens, __private::TokenStream as TokenStream2};
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, Token,
};

pub fn server_impl(
    args: proc_macro::TokenStream,
    s: TokenStream,
) -> TokenStream {
    let function: syn::ItemFn =
        match syn::parse(s).map_err(|e| e.to_compile_error()) {
            Ok(f) => f,
            Err(e) => return e.into(),
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
    struct_name: Option<Ident>,
    _comma: Option<Token![,]>,
    prefix: Option<Literal>,
    _comma2: Option<Token![,]>,
    encoding: Option<Literal>,
    _comma3: Option<Token![,]>,
    fn_path: Option<Literal>,
}

impl ToTokens for ServerFnArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let struct_name =
            self.struct_name.as_ref().map(|s| quote::quote! { #s, });
        let prefix = self.prefix.as_ref().map(|p| quote::quote! { #p, });
        let encoding = self.encoding.as_ref().map(|e| quote::quote! { #e, });
        let fn_path = self.fn_path.as_ref().map(|f| quote::quote! { #f });
        tokens.extend(quote::quote! {
            #struct_name
            #prefix
            #encoding
            #fn_path
        })
    }
}

impl Parse for ServerFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;
        let _comma = input.parse()?;
        let prefix = input.parse()?;
        let _comma2 = input.parse()?;
        let encoding = input.parse()?;
        let _comma3 = input.parse()?;
        let fn_path = input.parse()?;

        Ok(Self {
            struct_name,
            _comma,
            prefix,
            _comma2,
            encoding,
            _comma3,
            fn_path,
        })
    }
}
