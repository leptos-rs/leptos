use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn params_impl(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
    let name = &ast.ident;

    let fields_from_map = if let syn::Data::Struct(syn::DataStruct {
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
					span=> #ident: ::leptos_router::params::macro_helpers::Wrapper::<#ty>::__into_param(
                        map.get_str(#field_name_string),
                        #field_name_string
                    )?
				}
			})
            .collect()
    } else {
        vec![]
    };

    let fields_to_map = if let syn::Data::Struct(syn::DataStruct {
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
                    span=> if let Some(val) = ::leptos_router::params::macro_helpers::Wrapper::<#ty>::__param_to_string(&self.#ident, #field_name_string) {
                        map.insert(#field_name_string, val);
                    }
				}
			})
            .collect()
    } else {
        vec![]
    };

    let num_fields = fields_to_map.len();

    let gen = quote! {
        impl Params for #name {
            fn from_map(map: &::leptos_router::params::ParamsMap) -> ::core::result::Result<Self, ::leptos_router::params::ParamsError> {
                use ::leptos_router::params::macro_helpers::Fallback as _;

                Ok(Self {
                    #(#fields_from_map,)*
                })
            }

            fn to_map(&self) -> ::core::result::Result<::leptos_router::params::ParamsMap, ::leptos_router::params::ParamsError> {
                use ::leptos_router::params::macro_helpers::Fallback as _;
                let mut map = ::leptos_router::params::ParamsMap::with_capacity(#num_fields);
                #(#fields_to_map;)*
                Ok(map)
            }
        }
    };
    gen.into()
}
