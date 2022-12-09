// Based in large part on Dioxus: https://github.com/DioxusLabs/dioxus/blob/master/packages/core-macro/src/inlineprops.rs

#![allow(unstable_name_collisions)]

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt,};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *,
};
use itertools::Itertools;

pub struct InlinePropsBody {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub cx_token: Box<Pat>,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    // pub fields: FieldsNamed,
    pub output: ReturnType,
    pub where_clause: Option<WhereClause>,
    pub block: Box<Block>,
    pub doc_comment: String
}

/// The custom rusty variant of parsing rsx!
impl Parse for InlinePropsBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let paren_token = syn::parenthesized!(content in input);

        let first_arg: FnArg = content.parse()?;
        let cx_token = {
            match first_arg {
                FnArg::Receiver(_) => panic!("first argument must not be a receiver argument"),
                FnArg::Typed(f) => f.pat,
            }
        };

        let _: Result<Token![,]> = content.parse();

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output = input.parse()?;

        let where_clause = input
            .peek(syn::token::Where)
            .then(|| input.parse())
            .transpose()?;

        let block = input.parse()?;

        let doc_comment = attrs.iter().filter_map(|attr| if attr.path.segments[0].ident == "doc" {
            
            Some(attr.clone().tokens.into_iter().filter_map(|token| if let TokenTree::Literal(_) = token {
                // remove quotes
                let chars = token.to_string();
                let mut chars = chars.chars();
                chars.next();
                chars.next_back();
                Some(chars.as_str().to_string())
            } else {
                None
            }).collect::<String>())
            } else {
                None
            })
            .intersperse_with(|| "\n".to_string())
            .collect();

        Ok(Self {
            vis,
            fn_token,
            ident,
            generics,
            paren_token,
            inputs,
            output,
            where_clause,
            block,
            cx_token,
            attrs,
            doc_comment
        })
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for InlinePropsBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let Self {
            vis,
            ident,
            generics,
            inputs,
            output,
            where_clause,
            block,
            cx_token,
            attrs,
            doc_comment,
            ..
        } = self;

        let field_docs: HashMap<String, String> = {
            let mut map = HashMap::new();
            let mut pieces = doc_comment.split("# Props");
            pieces.next();
            let rest = pieces.next().unwrap_or_default();
            let mut current_field_name = String::new();
            let mut current_field_value = String::new();
            for line in rest.split('\n') {
                if let Some(line) = line.strip_prefix(" - ") {
                    let mut pieces = line.split("**");
                    pieces.next();
                    let field_name = pieces.next();
                    let field_value = pieces.next().unwrap_or_default();
                    let field_value = if let Some((_ty, desc)) = field_value.split_once('-') {
                        desc
                    } else {
                        field_value
                    };
                    if let Some(field_name) = field_name {
                        if !current_field_name.is_empty() {
                            map.insert(current_field_name.clone(), current_field_value.clone());
                        }
                        current_field_name = field_name.to_string();
                        current_field_value = String::new();
                        current_field_value.push_str(field_value);
                    } else  {
                        current_field_value.push_str(field_value);
                    }
                } else {
                    current_field_value.push_str(line);
                }
            }
            if !current_field_name.is_empty() {
                map.insert(current_field_name, current_field_value.clone());
            }

            map
        };

        let fields = inputs.iter().map(|f| {
            let typed_arg = match f {
                FnArg::Receiver(_) => todo!(),
                FnArg::Typed(t) => t,
            };
            let comment = if let Pat::Ident(ident) = &*typed_arg.pat {
                field_docs.get(&ident.ident.to_string()).cloned()
            } else {
                None
            }.unwrap_or_default();
            let comment_macro = quote! {
                #[doc = #comment]
            };
            if let Type::Path(pat) = &*typed_arg.ty {
                if pat.path.segments[0].ident == "Option" {
                    quote! {
                        #comment_macro
                        #[builder(default, setter(strip_option, doc = #comment))]
                        pub #f
                    }
                } else {
                    quote! {
                        #comment_macro
                        #[builder(setter(doc = #comment))]
                        pub #f
                    }
                }
            } else {
                quote! {
                    #comment
                    #vis #f
                }
            }
        });

        let struct_name = Ident::new(&format!("{}Props", ident), Span::call_site());
        let prop_struct_comments = format!("Props for the [`{ident}`] component.");

        let field_names = inputs.iter().filter_map(|f| match f {
            FnArg::Receiver(_) => todo!(),
            FnArg::Typed(t) => Some(&t.pat),
        });

        let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
            Some(lt)
        } else {
            None
        };

        //let modifiers = if first_lifetime.is_some() {
        let modifiers = quote! {
            #[derive(leptos::typed_builder::TypedBuilder)]
            #[builder(doc)]
        };
        /* } else {
            quote! { #[derive(Props, PartialEq, Eq)] }
        }; */

        let (_scope_lifetime, fn_generics, struct_generics) = if let Some(lt) = first_lifetime {
            let struct_generics: Punctuated<_, token::Comma> = generics
                .params
                .iter()
                .map(|it| match it {
                    GenericParam::Type(tp) => {
                        let mut tp = tp.clone();
                        tp.bounds.push(parse_quote!( 'a ));

                        GenericParam::Type(tp)
                    }
                    _ => it.clone(),
                })
                .collect();

            (
                quote! { #lt, },
                generics.clone(),
                quote! { <#struct_generics> },
            )
        } else {
            let fn_generics = generics.clone();

            (quote! { }, fn_generics, quote! { #generics })
        };

        out_tokens.append_all(quote! {
            #modifiers
            #[doc = #prop_struct_comments]
            #[allow(non_camel_case_types)]
            #vis struct #struct_name #struct_generics
            #where_clause
            {
                #(#fields),*
            }

            #[allow(non_snake_case)]
            #(#attrs)*
            #vis fn #ident #fn_generics (#cx_token: Scope, props: #struct_name #struct_generics) #output
            #where_clause
            {
                let #struct_name { #(#field_names),* } = props;
                #block
            }
        });
    }
}
