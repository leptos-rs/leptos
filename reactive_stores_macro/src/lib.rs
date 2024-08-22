use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, abort_call_site, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    token::Comma,
    Field, Fields, Generics, Ident, Index, Meta, Result, Token, Type, Variant,
    Visibility, WhereClause,
};

#[proc_macro_error]
#[proc_macro_derive(Store, attributes(store))]
pub fn derive_store(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as Model)
        .into_token_stream()
        .into()
}

#[proc_macro_error]
#[proc_macro_derive(Patch, attributes(store))]
pub fn derive_patch(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as PatchModel)
        .into_token_stream()
        .into()
}

struct Model {
    vis: Visibility,
    name: Ident,
    generics: Generics,
    ty: ModelTy,
}

enum ModelTy {
    Struct { fields: Vec<Field> },
    Enum { variants: Vec<Variant> },
}

impl Parse for Model {
    fn parse(input: ParseStream) -> Result<Self> {
        let input = syn::DeriveInput::parse(input)?;

        let ty = match input.data {
            syn::Data::Struct(s) => {
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

                ModelTy::Struct { fields }
            }
            syn::Data::Enum(e) => ModelTy::Enum {
                variants: e.variants.into_iter().collect(),
            },
            _ => {
                abort_call_site!(
                    "only structs and enums can be used with `Store`"
                );
            }
        };

        Ok(Self {
            vis: input.vis,
            generics: input.generics,
            name: input.ident,
            ty,
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

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let library_path = quote! { reactive_stores };
        let Model {
            vis,
            name,
            generics,
            ty,
        } = &self;
        let any_store_field = Ident::new("AnyStoreField", Span::call_site());
        let trait_name = Ident::new(&format!("{name}StoreFields"), name.span());
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
                            #any_store_field: #library_path::StoreField<Value = #name #generics>,
                            #predicates
                    }
                })
                .unwrap_or_else(|| quote! { where #any_store_field: #library_path::StoreField<Value = #name #generics> })
        };

        // define an extension trait that matches this struct
        // and implement that trait for all StoreFields
        let (trait_fields, read_fields): (Vec<_>, Vec<_>) =
            ty.to_field_data(&library_path, generics, &any_store_field, name);

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

impl ModelTy {
    fn to_field_data(
        &self,
        library_path: &TokenStream,
        generics: &Generics,
        any_store_field: &Ident,
        name: &Ident,
    ) -> (Vec<TokenStream>, Vec<TokenStream>) {
        match self {
            ModelTy::Struct { fields } => fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let Field {
                        ident, ty, attrs, ..
                    } = &field;
                    let modes = attrs
                        .iter()
                        .find_map(|attr| {
                            attr.meta.path().is_ident("store").then(|| {
                                match &attr.meta {
                                    Meta::List(list) => {
                                        match Punctuated::<
                                                SubfieldMode,
                                                Comma,
                                            >::parse_terminated
                                                .parse2(list.tokens.clone())
                                            {
                                                Ok(modes) => Some(
                                                    modes
                                                        .iter()
                                                        .cloned()
                                                        .collect::<Vec<_>>(),
                                                ),
                                                Err(e) => abort!(list, e),
                                            }
                                    }
                                    _ => None,
                                }
                            })
                        })
                        .flatten();

                    (
                        field_to_tokens(
                            idx,
                            false,
                            modes.as_deref(),
                            library_path,
                            ident.as_ref(),
                            generics,
                            any_store_field,
                            name,
                            ty,
                        ),
                        field_to_tokens(
                            idx,
                            true,
                            modes.as_deref(),
                            library_path,
                            ident.as_ref(),
                            generics,
                            any_store_field,
                            name,
                            ty,
                        ),
                    )
                })
                .unzip(),
            ModelTy::Enum { variants } => variants
                .iter()
                .enumerate()
                .map(|(idx, variant)| {
                    let Variant { ident, fields, .. } = variant;

                    (
                        variant_to_tokens(
                            idx,
                            false,
                            library_path,
                            ident,
                            generics,
                            any_store_field,
                            name,
                            fields,
                        ),
                        variant_to_tokens(
                            idx,
                            true,
                            library_path,
                            ident,
                            generics,
                            any_store_field,
                            name,
                            fields,
                        ),
                    )
                })
                .unzip(),
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
    name: &Ident,
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
            // TODO keyed_by
            let SubfieldMode::Keyed(_keyed_by, key_ty) = mode;
            let signature = quote! {
                fn #ident(self) ->  #library_path::KeyedField<#any_store_field, #name #generics, #ty, #key_ty>
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
            fn #ident(self) ->  #library_path::Subfield<#any_store_field, #name #generics, #ty> {
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
            fn #ident(self) ->  #library_path::Subfield<#any_store_field, #name #generics, #ty>;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn variant_to_tokens(
    idx: usize,
    include_body: bool,
    library_path: &proc_macro2::TokenStream,
    ident: &Ident,
    generics: &Generics,
    any_store_field: &Ident,
    name: &Ident,
    fields: &Fields,
) -> proc_macro2::TokenStream {
    // the method name will always be the snake_cased ident
    let orig_ident = &ident;
    let ident =
        Ident::new(&ident.to_string().to_case(Case::Snake), ident.span());

    match fields {
        // For unit enum fields, we will just return a `bool` subfield, which is
        // true when this field matches
        Fields::Unit => {
            // default subfield
            if include_body {
                quote! {
                    fn #ident(&self) -> bool {
                        match #library_path::StoreField::reader(self) {
                            Some(reader) => {
                                let path = #library_path::StoreField::path(self).into_iter().collect();
                                let trigger = #library_path::StoreField::get_trigger(self, path);
                                trigger.track();
                                matches!(&*reader, #name::#orig_ident)
                            },
                            None => false
                        }
                    }
                }
            } else {
                quote! {
                    fn #ident(&self) -> bool;
                }
            }
        }
        Fields::Named(_) => todo!(),
        Fields::Unnamed(_) => todo!(),
    }
}

struct PatchModel {
    pub name: Ident,
    pub generics: Generics,
    pub fields: Vec<Field>,
}

impl Parse for PatchModel {
    fn parse(input: ParseStream) -> Result<Self> {
        let input = syn::DeriveInput::parse(input)?;

        let syn::Data::Struct(s) = input.data else {
            abort_call_site!("only structs can be used with `Patch`");
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
            name: input.ident,
            generics: input.generics,
            fields,
        })
    }
}

impl ToTokens for PatchModel {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let library_path = quote! { reactive_stores };
        let PatchModel {
            name,
            generics,
            fields,
        } = &self;

        let fields = fields.iter().enumerate().map(|(idx, field)| {
            let field_name = match &field.ident {
                Some(ident) => quote! { #ident },
                None => quote! { #idx },
            };
            quote! {
                #library_path::PatchField::patch_field(
                    &mut self.#field_name,
                    new.#field_name,
                    &new_path,
                    notify
                );
                new_path.replace_last(#idx + 1);
            }
        });

        // read access
        tokens.extend(quote! {
            impl #library_path::PatchField for #name #generics
            {
                fn patch_field(
                    &mut self,
                    new: Self,
                    path: &#library_path::StorePath,
                    notify: &mut dyn FnMut(&#library_path::StorePath),
                ) {
                    let mut new_path = path.clone();
                    new_path.push(0);
                    #(#fields)*
                }
            }
        });
    }
}
