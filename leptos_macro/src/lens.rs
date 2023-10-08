use proc_macro2::Ident;
use quote::quote;
use syn::{spanned::Spanned, Data, Field};

fn named_field_method(field: &Field, name: &Ident) -> proc_macro2::TokenStream {
    let field_accessor = name;
    let field_ty = &field.ty;

    quote! {
        pub fn #name(
            signal: ::leptos::RwSignal<Self>,
        ) -> (::leptos::Signal<#field_ty>, ::leptos::SignalSetter<#field_ty>) {
            ::leptos::create_slice(
                signal,
                |st: &Self| st.#field_accessor.clone(),
                |st: &mut Self, n: #field_ty| st.#field_accessor = n
            )
        }
    }
}

fn unnamed_field_method(
    field: &Field,
    index: usize,
) -> proc_macro2::TokenStream {
    let number_ident = syn::Ident::new(&format!("_{index}"), field.span());
    let field_accessor = syn::Index::from(index);
    let field_ty = &field.ty;

    quote! {
        pub fn #number_ident(
            signal: ::leptos::RwSignal<Self>,
        ) -> (::leptos::Signal<#field_ty>, ::leptos::SignalSetter<#field_ty>) {
            ::leptos::create_slice(
                signal,
                |st: &Self| st.#field_accessor.clone(),
                |st: &mut Self, n: #field_ty| st.#field_accessor = n
            )
        }
    }
}

pub fn lens_impl(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let source_ident = &ast.ident;

    let Data::Struct(source_data) = &ast.data else {
        panic!("Lens cannot be derived by enums or unions")
    };

    let lens_methods =
        source_data
            .fields
            .iter()
            .enumerate()
            .map(|(field_index, field)| {
                if let Some(field_name) = &field.ident {
                    named_field_method(&field, field_name)
                } else {
                    unnamed_field_method(&field, field_index)
                }
            });

    quote! {

        impl #source_ident {
            #(#lens_methods)*
        }

    }
    .into()
}
