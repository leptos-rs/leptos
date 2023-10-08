use quote::quote;
use syn::Data;

pub fn lens_impl(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let source_ident = &ast.ident;

    let Data::Struct(source_data) = &ast.data else {
        panic!("Lens is only implemented on strcuts")
    };

    let lens_methods = source_data.fields.iter().map(|field| {
        let Some(field_ident) = &field.ident else {
            panic!(
                "Encountered unnamed field, Lens cannot be used on tuple \
                 structs"
            );
        };

        let field_ty = &field.ty;

        quote! {

            pub fn #field_ident(
                signal: ::leptos::RwSignal<Self>,
            ) -> (::leptos::Signal<#field_ty>, ::leptos::SignalSetter<#field_ty>) {
                ::leptos::create_slice(signal, |st: &Self| st.#field_ident, |st: &mut Self, n: #field_ty| st.#field_ident = n)
            }

        }
    });

    quote! {

        impl #source_ident {
            #(#lens_methods)*
        }

    }
    .into()
}
