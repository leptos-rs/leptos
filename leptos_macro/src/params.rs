use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn impl_params(ast: &syn::DeriveInput) -> proc_macro::TokenStream {
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
				let field_name_string = &field.ident.as_ref().expect("expected named struct fields").to_string();
				let ident = &field.ident;
				let ty = &field.ty;
				let span = field.span();

				quote_spanned! {
					span => #ident: <#ty>::into_param(map.get(#field_name_string).map(|n| n.as_str()), #field_name_string)?
				}
			})
            .collect()
    } else {
        vec![]
    };

    let gen = quote! {
        impl Params for #name {
            fn from_map(map: &::leptos_router::ParamsMap) -> Result<Self, ::leptos_router::ParamsError> {
                Ok(Self {
                    #(#fields,)*
                })
            }
        }
    };
    gen.into()
}
