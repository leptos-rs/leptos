use proc_macro2::TokenStream;
use quote::{format_ident, ToTokens};
use syn::spanned::Spanned;

use super::{
    parsing::{Field, ModeKind},
    Model,
};

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

        todo!();

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

struct Impl {
    generics: syn::Generics,
    name: syn::Ident,
    inner: TokenStream,
}

impl ToTokens for Impl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            generics,
            name,
            inner,
        } = self;

        let (impl_types, struct_types, where_clause) =
            generics.split_for_impl();

        let s = quote! {
            impl #impl_types #name #struct_types #where_clause {
                #inner
            }
        };

        tokens.extend(s)
    }
}

struct IntoSignalFunction {
    vis: syn::Visibility,
    name: syn::Ident,
    is_tuple_struct: bool,
    mode: ModeKind,
    fields: Vec<Field>,
}

impl ToTokens for IntoSignalFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            vis,
            name,
            is_tuple_struct,
            mode,
            fields,
        } = self;

        let is_tuple_struct = *is_tuple_struct;

        let fn_name = match mode {
            ModeKind::Signal => {
                format_ident!("into_signals", span = name.span())
            }
            ModeKind::RwSignal => {
                format_ident!("into_rw_signals", span = name.span())
            }
            ModeKind::Store => {
                format_ident!("into_stored_values", span = name.span())
            }
        };

        let return_ty = match mode {
            ModeKind::Signal => {
                let read_name = format_ident!("{name}Read");
                let write_name = format_ident!("{name}Write");

                quote! { (#read_name, #write_name) }
            }
            ModeKind::RwSignal => {
                let rw_name = format_ident!("{name}Rw");

                quote! { #rw_name }
            }
            ModeKind::Store => {
                let stored_name = format_ident!("{name}Stored");

                quote! { #stored_name }
            }
        };

        let self_fields = field_names(fields);
        let self_fields = wrap_with_struct_or_tuple_delimiters(
            is_tuple_struct,
            quote! { #(#self_fields),* },
        );

        let signal_fn = match mode {
            ModeKind::Signal => quote! { create_signal },
            ModeKind::RwSignal => quote! { create_rw_signal },
            ModeKind::Store => quote! { store_value },
        };

        let field_names_ = field_names(fields);

        let signal_field_names = match mode {
            ModeKind::Signal => field_names(fields)
                .map(|name| {
                    let read_name = format_ident!("{name}_read");
                    let write_name = format_ident!("{name}_write");

                    quote! { (#read_name, #write_name) }
                })
                .collect::<Vec<_>>(),
            ModeKind::RwSignal | ModeKind::Store => {
                { field_names(fields).map(|name| quote! { #name }) }
                    .collect::<Vec<_>>()
            }
        };

        let return_value = match mode {
            ModeKind::Signal => {
                let read_struct = format_ident!("{name}Read");
                let write_struct = format_ident!("{name}Write");

                let read_fields = field_names(fields).map(|name| {
                    let read_name = format_ident!("read_{name}");

                    if is_tuple_struct {
                        quote! { #read_name }
                    } else {
                        quote! { #name: read_name }
                    }
                });
                let read_fields = wrap_with_struct_or_tuple_delimiters(
                    is_tuple_struct,
                    quote! { #(#read_fields),* },
                );

                let write_fields = field_names(fields).map(|name| {
                    let write_name = format_ident!("write_{name}");

                    if is_tuple_struct {
                        quote! { #write_name }
                    } else {
                        quote! { #name: #write_name }
                    }
                });
                let write_fields = wrap_with_struct_or_tuple_delimiters(
                    is_tuple_struct,
                    quote! { #(#write_fields),* },
                );

                quote! {
                    (
                        #read_struct #read_fields,
                        #write_struct #write_fields,
                    )
                }
            }
            ModeKind::RwSignal => {
                let rw_name = format_ident!("{name}Rw");

                let rw_fields = field_names(fields);
                let rw_fields = wrap_with_struct_or_tuple_delimiters(
                    is_tuple_struct,
                    quote! { #(#rw_fields),* },
                );

                quote! {
                    #rw_name #rw_fields
                }
            }
            ModeKind::Store => {
                let stored_name = format_ident!("{name}Stored");

                let stored_fields = field_names(fields);
                let stored_fields = wrap_with_struct_or_tuple_delimiters(
                    is_tuple_struct,
                    quote! { #(#stored_fields),* },
                );

                quote! {
                    #stored_name #stored_fields
                }
            }
        };

        let s = quote! {
            #vis fn #fn_name(self) -> #return_ty {
                let Self #self_fields = self;

                #(
                    let #signal_field_names
                        = ::leptos::leptos_reactive::#signal_fn(#field_names_);
                )*

                #return_value
            }
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
