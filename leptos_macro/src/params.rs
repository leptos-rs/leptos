use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn params_impl(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &ast.ident;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(ref fields),
        ..
    }) = ast.data
    {
        fields
            .named
            .iter()
            .map(|field| {
				let field_name_string = &field
                    .ident
                    .as_ref()
                    .expect("expected named struct fields")
                    .to_string()
                    .trim_start_matches("r#")
                    .to_owned();
				let ident = &field.ident;
				let ty = &field.ty;
				let span = field.span();

				quote_spanned! {
					span=> #ident: <#ty as ::leptos_router::params::IntoParam>::into_param(
                        map.get_str(#field_name_string),
                        #field_name_string
                    )?
				}
			})
            .collect()
    } else {
        vec![]
    };

    let gen = quote! {
        impl Params for #name {
            fn from_map(map: &::leptos_router::params::ParamsMap) -> ::core::result::Result<Self, ::leptos_router::params::ParamsError> {
                Ok(Self {
                    #(#fields,)*
                })
            }
        }
    };
    gen.into()
}
