use proc_macro2::TokenStream;
use quote::{format_ident, ToTokens};
use syn::spanned::Spanned;

use super::{parsing::Field, Model};

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let read_struct: Struct = (SignalKind::ReadSignal, self).into();
        let write_struct: Struct = (SignalKind::WriteSignal, self).into();
        let rw_struct: Struct = (SignalKind::RwSignal, self).into();
        let stored_struct: Struct = (SignalKind::StoredValue, self).into();

        let read_write_structs = self.modes.signal.then_some(quote! {
            #read_struct
            #write_struct
        });
        let rw_struct = self.modes.rw_signal.then_some(quote! { #rw_struct });
        let stored_struct =
            self.modes.store.then_some(quote! { #stored_struct });

        let s = quote! {
            #read_write_structs

            #rw_struct

            #stored_struct
        };

        tokens.extend(s)
    }
}

struct Struct {
    vis: syn::Visibility,
    generics: syn::Generics,
    name: syn::Ident,
    signal_kind: SignalKind,
    is_tuple_struct: bool,
    fields: Vec<StructField>,
}

impl From<(SignalKind, &Model)> for Struct {
    fn from((signal_kind, model): (SignalKind, &Model)) -> Self {
        Self {
            vis: model.vis.to_owned(),
            generics: model.generics.to_owned(),
            name: model.struct_name.to_owned(),
            signal_kind,
            is_tuple_struct: model.is_tuple_struct,
            fields: model
                .fields
                .iter()
                .map(|field| (model.vis.to_owned(), signal_kind, field).into())
                .collect(),
        }
    }
}

impl ToTokens for Struct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            vis,
            generics,
            name,
            signal_kind,
            is_tuple_struct,
            fields,
        } = self;

        let is_tuple_struct = *is_tuple_struct;

        let (_, generic_types, where_clause) = generics.split_for_impl();

        let struct_where_clause =
            (!is_tuple_struct).then_some(quote! { #where_clause });
        let tuple_where_clause =
            is_tuple_struct.then_some(quote! { #where_clause });

        let name = match signal_kind {
            SignalKind::ReadSignal => format_ident!("{name}Read"),
            SignalKind::WriteSignal => format_ident!("{name}Write"),
            SignalKind::RwSignal => format_ident!("{name}Rw"),
            SignalKind::StoredValue => format_ident!("{name}Stored"),
        };

        let fields = fields.iter().map(ToTokens::to_token_stream);
        let fields = wrap_with_struct_or_tuple_delimiters(
            is_tuple_struct,
            quote! { #(#fields),* },
        );

        let semi_token = is_tuple_struct.then_some(quote!(;));

        let s = quote! {
            #[derive(Clone, Copy)]
            #vis struct #name #generic_types
            #struct_where_clause
                #fields
                #tuple_where_clause #semi_token
        };

        tokens.extend(s)
    }
}

struct StructField {
    vis: syn::Visibility,
    signal_kind: SignalKind,
    field: Field,
}

impl From<(syn::Visibility, SignalKind, &Field)> for StructField {
    fn from(
        (vis, signal_kind, field): (syn::Visibility, SignalKind, &Field),
    ) -> Self {
        Self {
            vis,
            signal_kind,
            field: field.to_owned(),
        }
    }
}

impl ToTokens for StructField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            vis,
            signal_kind,
            field,
        } = self;

        let field = match field {
            Field::Named { name, ty } => quote! { #name: #signal_kind<#ty> },
            Field::Unnamed(ty) => quote! { #ty },
        };

        let s = quote! { #vis #field };

        tokens.extend(s)
    }
}

#[derive(Clone, Copy)]
enum SignalKind {
    ReadSignal,
    WriteSignal,
    RwSignal,
    StoredValue,
}

impl ToTokens for SignalKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let prefix = quote! { ::leptos::leptos_reactive };

        let s = match self {
            SignalKind::ReadSignal => quote! { #prefix::ReadSignal },
            SignalKind::WriteSignal => quote! { #prefix::WriteSignal },
            SignalKind::RwSignal => quote! { #prefix::RwSignal },
            SignalKind::StoredValue => quote! { #prefix::StoredValue },
        };

        tokens.extend(s)
    }
}

fn wrap_with_struct_or_tuple_delimiters(
    is_tuple_struct: bool,
    inner: impl ToTokens,
) -> TokenStream {
    if is_tuple_struct {
        quote! { { #inner } }
    } else {
        quote! { (#inner) }
    }
}

fn field_names(fields: &[Field]) -> impl Iterator<Item = syn::Ident> + '_ {
    fields.iter().enumerate().map(|(i, field)| match field {
        Field::Named { name, .. } => name.to_owned(),
        Field::Unnamed(ty) => quote::format_ident!("_{i}", span = ty.span()),
    })
}
