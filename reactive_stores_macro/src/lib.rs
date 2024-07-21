use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, abort_call_site, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    token::Comma,
    Data, Field, Fields, Generics, Ident, Index, Meta, MetaList, Result, Token,
    Type, Visibility, WhereClause,
};

#[proc_macro_error]
#[proc_macro_derive(Store, attributes(store))]
pub fn derive_store(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as Model)
        .into_token_stream()
        .into()
}

struct Model {
    pub vis: Visibility,
    pub struct_name: Ident,
    pub generics: Generics,
    pub fields: Vec<Field>,
}

impl Parse for Model {
    fn parse(input: ParseStream) -> Result<Self> {
        let input = syn::DeriveInput::parse(input)?;

        let syn::Data::Struct(s) = input.data else {
            abort_call_site!("only structs can be used with `Store`");
        };

        let fields = match s.fields {
            syn::Fields::Unit => {
                abort!(s.semi_token, "unit structs are not supported");
            }
            syn::Fields::Named(fields) => {
                fields.named.into_iter().collect::<Vec<_>>()
            }
            syn::Fields::Unnamed(fields) => {
                fields.unnamed.into_iter().collect::<Vec<_>>()
            }
        };

        Ok(Self {
            vis: input.vis,
            struct_name: input.ident,
            generics: input.generics,
            fields,
        })
    }
}

#[derive(Clone)]
enum SubfieldMode {
    Keyed(Ident, Type),
}

impl Parse for SubfieldMode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mode: Ident = input.parse()?;
        if mode == "key" {
            let _eq: Token!(=) = input.parse()?;
            let ident: Ident = input.parse()?;
            let _col: Token!(:) = input.parse()?;
            let ty: Type = input.parse()?;
            Ok(SubfieldMode::Keyed(ident, ty))
        } else {
            Err(input.error("expected `key = <ident>: <Type>`"))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn field_to_tokens(
    idx: usize,
    include_body: bool,
    modes: Option<&[SubfieldMode]>,
    library_path: &proc_macro2::TokenStream,
    orig_ident: Option<&Ident>,
    generics: &Generics,
    any_store_field: &Ident,
    struct_name: &Ident,
    ty: &Type,
) -> proc_macro2::TokenStream {
    let ident = if orig_ident.is_none() {
        let idx = Ident::new(&format!("field{idx}"), Span::call_site());
        quote! { #idx }
    } else {
        quote! { #orig_ident }
    };
    let locator = if orig_ident.is_none() {
        let idx = Index::from(idx);
        quote! { #idx }
    } else {
        quote! { #ident }
    };

    if let Some(modes) = modes {
        if modes.len() == 1 {
            let mode = &modes[0];
            // Can replace with a match if additional modes added
            let SubfieldMode::Keyed(keyed_by, key_ty) = mode;
            let signature = quote! {
                fn #ident(self) ->  #library_path::KeyedField<#any_store_field, #struct_name #generics, #ty, #key_ty>
            };
            return if include_body {
                quote! {
                    #signature {
                        todo!()
                    }
                }
            } else {
                quote! { #signature; }
            };
        } else {
            abort!(
                orig_ident
                    .map(|ident| ident.span())
                    .unwrap_or_else(Span::call_site),
                "multiple modes not currently supported"
            );
        }
    }

    // default subfield
    if include_body {
        quote! {
            fn #ident(self) ->  #library_path::Subfield<#any_store_field, #struct_name #generics, #ty> {
                #library_path::Subfield::new(
                    self,
                    #idx.into(),
                    |prev| &prev.#locator,
                    |prev| &mut prev.#locator,
                )
            }
        }
    } else {
        quote! {
            fn #ident(self) ->  #library_path::Subfield<#any_store_field, #struct_name #generics, #ty>;
        }
    }
}

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let library_path = quote! { reactive_stores };
        let Model {
            vis,
            struct_name,
            generics,
            fields,
        } = &self;
        let any_store_field = Ident::new("AnyStoreField", Span::call_site());
        let trait_name = Ident::new(
            &format!("{struct_name}StoreFields"),
            struct_name.span(),
        );
        let generics_with_orig = {
            let params = &generics.params;
            quote! { <#any_store_field, #params> }
        };
        let where_with_orig = {
            generics
                .where_clause
                .as_ref()
                .map(|w| {
                    let WhereClause {
                        where_token,
                        predicates,
                    } = &w;
                    quote! {
                        #where_token
                            #any_store_field: #library_path::StoreField<#struct_name #generics>,
                            #predicates
                    }
                })
                .unwrap_or_else(|| quote! { where #any_store_field: #library_path::StoreField<#struct_name #generics> })
        };

        // define an extension trait that matches this struct
        let all_field_data = fields.iter().enumerate().map(|(idx, field)| {
            let Field { ident, ty, attrs, .. } = &field;
            let modes = attrs.iter().find_map(|attr| {
                attr.meta.path().is_ident("store").then(|| {
                    match &attr.meta {
                        Meta::List(list) => {
                            match Punctuated::<SubfieldMode, Comma>::parse_terminated.parse2(list.tokens.clone()) {
                                Ok(modes) => Some(modes.iter().cloned().collect::<Vec<_>>()),
                                Err(e) => abort!(list, e)
                            }
                        },
                        _ => None
                    }
                })
            }).flatten();

            (
                field_to_tokens(idx, false, modes.as_deref(), &library_path, ident.as_ref(), generics, &any_store_field, struct_name, ty),
                field_to_tokens(idx, true, modes.as_deref(), &library_path, ident.as_ref(), generics, &any_store_field, struct_name, ty),
            )
        });

        // implement that trait for all StoreFields
        let (trait_fields, read_fields): (Vec<_>, Vec<_>) =
            all_field_data.unzip();

        // read access
        tokens.extend(quote! {
            #vis trait #trait_name <AnyStoreField>
            #where_with_orig
            {
                #(#trait_fields)*
            }

            impl #generics_with_orig #trait_name <AnyStoreField> for AnyStoreField
            #where_with_orig
            {
               #(#read_fields)*
            }
        });
    }
}
