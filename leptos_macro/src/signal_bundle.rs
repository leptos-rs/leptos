use syn::parse::Parse;

pub struct Model {
    modes: Modes,
    vis: syn::Visibility,
    struct_name: syn::Ident,
    generics: syn::Generics,
    is_tuple_struct: bool,
    fields: Vec<Field>,
}

impl Parse for Model {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let input = syn::DeriveInput::parse(input)?;

        let modes = input
            .attrs
            .into_iter()
            .filter(|attr| attr.meta.path().is_ident("bundle"))
            .map(|attr| attr.parse_args::<Modes>())
            .collect::<Result<Modes, _>>()?;

        let syn::Data::Struct(s) = input.data else {
            abort_call_site!("only structs can be used with `SignalBundle`");
        };

        let (is_tuple_struct, fields) = match s.fields {
            syn::Fields::Unit => {
                abort!(s.semi_token, "unit structs are not supported");
            }
            syn::Fields::Named(fields) => (
                false,
                fields.named.into_iter().map(Into::into).collect::<Vec<_>>(),
            ),
            syn::Fields::Unnamed(fields) => (
                true,
                fields
                    .unnamed
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            ),
        };

        Ok(Self {
            modes,
            vis: input.vis,
            struct_name: input.ident,
            generics: input.generics,
            is_tuple_struct,
            fields,
        })
    }
}

impl quote::ToTokens for Model {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let signal_bundle = self.generate_signal_bundle();
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
    fn generate_signal_bundle(&self) -> Option<proc_macro2::TokenStream> {
        if !self.modes.signal {
            return None;
        }

        let Self {
            modes: _,
            vis,
            struct_name,
            generics,
            is_tuple_struct,
            fields,
        } = self;

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
            let fields = fields.into_iter().map(|field| {
                field.to_tokens_with_mode(FieldModeKind::ReadSignal)
            });

            if self.is_tuple_struct {
                quote!((#(#fields),*))
            } else {
                quote!({ #(#fields),* })
            }
        };

        let write_fields = {
            let fields = fields.into_iter().map(|field| {
                field.to_tokens_with_mode(FieldModeKind::WriteSignal)
            });

            if self.is_tuple_struct {
                quote!((#(#fields),*))
            } else {
                quote!({ #(#fields),* })
            }
        };

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
            #read_fields
            #tuple_where_clause
            #tuple_semi_token

          impl #impl_generic_types #struct_name #generic_types #where_clause {
            #vis fn into_signal_bundle(self) -> (#read_name, #write_name) {
              todo!()
            }
          }
        })
    }

    fn generate_rw_signal_bundle(&self) -> Option<proc_macro2::TokenStream> {
        todo!()
    }

    fn generate_store_bundle(&self) -> Option<proc_macro2::TokenStream> {
        todo!()
    }
}

#[derive(Default)]
struct Modes {
    /// Generates seperate read/write structs.
    signal: bool,
    /// Generates single read/write struct.
    rw_signal: bool,
    /// Generates stored value struct.
    store: bool,
}

impl std::ops::BitOr for Modes {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.signal |= rhs.signal;
        self.rw_signal |= rhs.rw_signal;
        self.store |= rhs.store;

        self
    }
}

impl std::ops::BitOr<ModeKind> for Modes {
    type Output = Self;

    fn bitor(mut self, mode: ModeKind) -> Self::Output {
        match mode {
            ModeKind::Signal => self.signal = true,
            ModeKind::RwSignal => self.rw_signal = true,
            ModeKind::Store => self.store = true,
        }

        self
    }
}

impl FromIterator<Self> for Modes {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Self::default(), std::ops::BitOr::bitor)
    }
}

impl Parse for Modes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let this = syn::punctuated::Punctuated::<
          ModeKind,
          syn::Token![,]
        >::parse_terminated(input)?
        .into_iter()
        .fold(Self::default(), std::ops::BitOr::bitor);

        Ok(this)
    }
}

enum ModeKind {
    Signal,
    RwSignal,
    Store,
}

impl Parse for ModeKind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;

        let this = match ident.to_string().as_str() {
            "signal" => Self::Signal,
            "rw_signal" => Self::RwSignal,
            "store" => Self::Store,
            _ => abort!(
              ident, "unknown argument to `#[bundle()]`";
              hint = "must be any of `signal`, `rw_signal`, and `store`"
            ),
        };

        Ok(this)
    }
}

enum Field {
    Named { name: syn::Ident, ty: syn::Type },
    Unnamed(syn::Type),
}

impl From<syn::Field> for Field {
    fn from(field: syn::Field) -> Self {
        if let Some(name) = field.ident {
            Self::Named { name, ty: field.ty }
        } else {
            Self::Unnamed(field.ty)
        }
    }
}

impl Field {
    fn to_tokens_with_mode(
        &self,
        mode: FieldModeKind,
    ) -> proc_macro2::TokenStream {
        match self {
            Field::Named { name, ty } => quote! { #ty: #mode<#ty> },
            Field::Unnamed(ty) => quote! { #mode<#ty> },
        }
    }
}

enum FieldModeKind {
    ReadSignal,
    WriteSignal,
    RwSignal,
    Store,
}

impl quote::ToTokens for FieldModeKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
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
