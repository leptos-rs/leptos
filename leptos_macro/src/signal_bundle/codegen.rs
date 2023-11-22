use super::parsing::{Field, Model};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;

impl ToTokens for Model {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let signal_bundle = generate_signal_bundle(self);
        let rw_signal_bundle = self.generate_rw_signal_bundle();
        let store_bundle = self.generate_store_bundle();

        let tokens_ = quote! {
          #signal_bundle
          #rw_signal_bundle
          #store_bundle
        };

        tokens.extend(tokens_);
    }
}

impl Model {
    fn generate_rw_signal_bundle(&self) -> Option<TokenStream> {
        todo!()
    }

    fn generate_store_bundle(&self) -> Option<TokenStream> {
        todo!()
    }
}

enum FieldModeKind {
    ReadSignal,
    WriteSignal,
    RwSignal,
    Store,
}

impl ToTokens for FieldModeKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty_prefix = quote! { ::leptos::leptos_reactive };

        let tokens_ = match self {
            FieldModeKind::ReadSignal => quote! { #ty_prefix::ReadSignal },
            FieldModeKind::WriteSignal => quote! { #ty_prefix::WriteSignal },
            FieldModeKind::RwSignal => quote! { #ty_prefix::RwSignal },
            FieldModeKind::Store => quote! { #ty_prefix::StoredValue },
        };

        tokens.extend(tokens_);
    }
}

fn generate_signal_bundle(model: &Model) -> Option<TokenStream> {
    if !model.modes.signal {
        return None;
    }

    let Model {
        modes: _,
        vis,
        struct_name,
        generics,
        is_tuple_struct,
        fields,
    } = model;

    let is_tuple_struct = *is_tuple_struct;

    let (impl_generic_types, generic_types, where_clause) =
        generics.split_for_impl();

    let tuple_where_clause = is_tuple_struct
        .then_some(quote! { #where_clause })
        .unwrap_or_default();
    let struct_where_clause = (!is_tuple_struct)
        .then_some(quote! { #where_clause })
        .unwrap_or_default();
    let tuple_semi_token = is_tuple_struct.then_some(quote!(;));

    let read_name = quote::format_ident!("{struct_name}Read");
    let write_name = quote::format_ident!("{struct_name}Write");

    let read_fields = {
        let fields = fields
            .iter()
            .zip(field_names(fields))
            .zip(field_types(fields))
            .map(|((field, name), ty)| {
                (
                    field,
                    name,
                    wrap_with_signal_type(FieldModeKind::ReadSignal, ty),
                )
            })
            .map(|(field, name, ty)| {
                if matches!(field, Field::Named { .. }) {
                    quote! { #name: #ty }
                } else {
                    quote! { #ty }
                }
            });

        wrap_with_struct_or_tuple_braces(
            is_tuple_struct,
            quote! { #(#fields),* },
        )
    };

    let write_fields = {
        let fields = fields
            .iter()
            .zip(field_names(fields))
            .zip(field_types(fields))
            .map(|((field, name), ty)| {
                (
                    field,
                    name,
                    wrap_with_signal_type(FieldModeKind::WriteSignal, ty),
                )
            })
            .map(|(field, name, ty)| {
                if matches!(field, Field::Named { .. }) {
                    quote! { #name: #ty }
                } else {
                    quote! { #ty }
                }
            });

        wrap_with_struct_or_tuple_braces(
            is_tuple_struct,
            quote! { #(#fields),* },
        )
    };

    let self_fields = field_names(fields);
    let self_fields = wrap_with_struct_or_tuple_braces(
        is_tuple_struct,
        quote! { #(#self_fields),* },
    );

    let field_names_ = field_names(fields);

    let read_field_names =
        field_names(fields).map(|name| quote::format_ident!("read_{name}"));

    let write_field_names =
        field_names(fields).map(|name| quote::format_ident!("write_{name}"));

    let read_output_fields =
        fields.iter().zip(field_names(fields)).map(|(field, name)| {
            if matches!(field, Field::Named { .. }) {
                let read_name = quote::format_ident!("read_{name}");

                quote! { #name: #read_name }
            } else {
                quote! { #name }
            }
        });
    let read_output_fields = wrap_with_struct_or_tuple_braces(
        is_tuple_struct,
        quote! { #(#read_output_fields),* },
    );

    Some(quote! {
      #[derive(Clone, Copy)]
      #vis struct #read_name
        #generic_types
        #struct_where_clause
        #read_fields
        #tuple_where_clause
        #tuple_semi_token

      #[derive(Clone, Copy)]
      #vis struct #write_name
        #generic_types
        #struct_where_clause
        #write_fields
        #tuple_where_clause
        #tuple_semi_token

      impl #impl_generic_types #struct_name #generic_types #where_clause {
        #vis fn into_signal_bundle(self) -> (#read_name, #write_name) {
          let Self #self_fields = self;
        }
        #(
            let (#read_field_names, #write_field_names)
                = ::leptos::leptos_reactive::create_signal(#field_names_);
        )*

        (
            #read_name #read_output_fields,
            // #write_name #write_output_fields,
        )
      }
    })
}

/// Wraps `inner` with `()` or `{}` depending if it's a tuple
/// struct or not.
fn wrap_with_struct_or_tuple_braces(
    is_tuple_struct: bool,
    inner: impl ToTokens,
) -> TokenStream {
    if is_tuple_struct {
        quote! { ( #inner ) }
    } else {
        quote! { { #inner } }
    }
}

/// Produces an iterator of field names.
fn field_names(fields: &[Field]) -> impl Iterator<Item = TokenStream> + '_ {
    fields.iter().enumerate().map(|(i, field)| match field {
        Field::Named { name, .. } => quote! { #name },
        Field::Unnamed(ty) => {
            let field_name = quote::format_ident!("_{i}", span = ty.span());

            quote! { #field_name }
        }
    })
}

/// Produces an iterator of field types.
fn field_types(fields: &[Field]) -> impl Iterator<Item = TokenStream> + '_ {
    fields.iter().map(|field| match field {
        Field::Named { ty, .. } => quote! { #ty },
        Field::Unnamed(ty) => quote! { #ty },
    })
}

/// Wraps a type with the appropriate signal type.
fn wrap_with_signal_type(
    mode: FieldModeKind,
    ty: impl ToTokens,
) -> TokenStream {
    quote! { #mode<#ty> }
}
