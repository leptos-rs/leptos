// Credit to Dioxus: https://github.com/DioxusLabs/dioxus/blob/master/packages/core-macro/src/Server.rs

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *, token::Type,
};

pub fn server_macro_impl(args: proc_macro::TokenStream, s: TokenStream2) -> Result<TokenStream2> {
    let ServerFnName { struct_name } = syn::parse::<ServerFnName>(args)?;
    let body = syn::parse::<ServerFnBody>(s.into())?;
    let fn_name = &body.ident;
    let fn_name_as_str = body.ident.to_string();
    let vis = body.vis;
    let block = body.block;

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

    let as_form_data_fields = field_names
        .clone()
        .map(|field_name| {
            let field_name_as_string = match (**field_name).clone() {
                Pat::Ident(id) => id.ident,
                _ => panic!("field names need to be identifiers"),
            };
            let field_name_as_string = field_name_as_string.to_string();
            quote::quote! {
                (#field_name_as_string, self.#field_name.to_json().expect("could not serialize field"))
            }
        })
        .collect::<Vec<_>>();

    let from_form_data_fields =  body.inputs.iter()
        .map(|field| {
            let (field_name, field_type) = match field {
                FnArg::Receiver(_) => panic!("cannot use receiver types in server function macro"),
                FnArg::Typed(t) => (t.pat.clone(), t.ty.clone()),
            };
            let field_name = match *field_name {
                Pat::Ident(id) => id.ident,
                _ => panic!("field names need to be identifiers"),
            };
            let field_name_as_string = field_name.to_string();
            quote::quote! {
                #field_name: data.iter()
                    .find(|(k, _)| k == #field_name_as_string)
                    .ok_or_else(|| ::leptos::ServerFnError::MissingArg(#field_name_as_string.into()))
                    .and_then(|(_, v)| #field_type::from_json(&v).map_err(|e| ::leptos::ServerFnError::Args(e.to_string())))?
                    
            }
        })
        .collect::<Vec<_>>();

    let field_names_2 = field_names.clone();
    let field_names_3 = field_names.clone();

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
        pub struct #struct_name {
            #(#fields),*
        }

        #[async_trait]
        impl ServerFn for #struct_name {
            type Output = #output_ty;

            fn url() -> &'static str {
                #fn_name_as_str
            }

            fn as_form_data(&self) -> Vec<(&'static str, String)> {
                vec![
                    #(#as_form_data_fields),*
                ]
            }

            fn from_form_data(data: &[u8]) -> Result<Self, ServerFnError> {
                let data = ::leptos::leptos_server::form_urlencoded::parse(data).collect::<Vec<_>>();
                Ok(Self {
                    #(#from_form_data_fields),*
                })
            }

            #[cfg(feature = "ssr")]
            async fn call_fn(self) -> Result<Self::Output, ServerFnError> {
                let #struct_name { #(#field_names),* } = self;
                #fn_name( #(#field_names_2),*).await
            }
        }

        #[cfg(feature = "ssr")]
        #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
            #block
        }
        #[cfg(not(feature = "ssr"))]
        #vis async fn #fn_name(#(#fn_args_2),*) #output_arrow #return_ty {
            ::leptos::call_server_fn(#struct_name::url(), #struct_name { #(#field_names_3),* }).await
        }
    })
}

pub struct ServerFnName {
    struct_name: Ident,
}

impl Parse for ServerFnName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;

        Ok(Self { struct_name })
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
/*
/// Serialize the same way, regardless of flavor
impl ToTokens for ServerFnBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let Self {
            vis,
            ident,
            generics,
            inputs,
            output,
            where_clause,
            block,
            attrs,
            ..
        } = self;

        let fields = inputs.iter().map(|f| {
            let typed_arg = match f {
                FnArg::Receiver(_) => todo!(),
                FnArg::Typed(t) => t,
            };
            if let Type::Path(pat) = &*typed_arg.ty {
                if pat.path.segments[0].ident == "Option" {
                    quote! {
                        #[builder(default, setter(strip_option))]
                        #vis #f
                    }
                } else {
                    quote! { #vis #f }
                }
            } else {
                quote! { #vis #f }
            }
        });

        let struct_name = Ident::new(&format!("{}Props", ident), Span::call_site());

        let field_names = inputs.iter().filter_map(|f| match f {
            FnArg::Receiver(_) => todo!(),
            FnArg::Typed(t) => Some(&t.pat),
        });

        let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
            Some(lt)
        } else {
            None
        };

        out_tokens.append_all(quote! {
            #[derive(Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
            pub struct #struct_name {}

            #[async_trait]
            impl ServerFn for #struct_name {
                type Output = i32;

                fn url() -> &'static str {
                    "get_server_count"
                }

                fn as_form_data(&self) -> Vec<(&'static str, String)> {
                    vec![]
                }

                #[cfg(feature = "ssr")]
                async fn call_fn(self) -> Result<Self::Output, ServerFnError> {
                    get_server_count().await
                }
            }

            #[cfg(feature = "ssr")]
            pub async fn get_server_count() -> Result<i32, ServerFnError> {
                Ok(COUNT.load(Ordering::Relaxed))
            }
            #[cfg(not(feature = "ssr"))]
            pub async fn get_server_count() -> Result<i32, ServerFnError> {
                call_server_fn(#struct_name::url(), #struct_name {}).await
            }
            #[cfg(not(feature = "ssr"))]
            pub async fn get_server_count_helper(args: #struct_name) -> Result<i32, ServerFnError> {
                call_server_fn(#struct_name::url(), args).await
            }
        });
    }
}

/*




*/
 */
