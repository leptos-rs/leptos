use syn::parse::Parse;

pub struct Model {
    pub modes: Modes,
    pub vis: syn::Visibility,
    pub struct_name: syn::Ident,
    pub generics: syn::Generics,
    pub is_tuple_struct: bool,
    pub fields: Vec<Field>,
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

#[derive(Default)]
pub struct Modes {
    /// Generates seperate read/write structs.
    pub signal: bool,
    /// Generates single read/write struct.
    pub rw_signal: bool,
    /// Generates stored value struct.
    pub store: bool,
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

pub enum Field {
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
