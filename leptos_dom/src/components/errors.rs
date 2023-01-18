use crate::{IntoView, Transparent};
use std::{error::Error, sync::Arc};

/// A struct to hold all the possible errors that could be provided by child Views
#[derive(Debug, Clone, Default)]
pub struct Errors(pub Vec<Arc<dyn Error>>);

impl IntoView for Errors {
  fn into_view(self, cx: leptos_reactive::Scope) -> crate::View {
    Transparent::new(self).into_view(cx)
  }
}
