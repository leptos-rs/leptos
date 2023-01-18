use leptos_reactive::{use_context, RwSignal};

use crate::{HydrationCtx, HydrationKey, IntoView, Transparent};
use std::{collections::HashMap, error::Error, rc::Rc};

/// A struct to hold all the possible errors that could be provided by child Views
#[derive(Debug, Clone, Default)]
pub struct Errors(pub HashMap<HydrationKey, Rc<dyn Error>>);

impl<T, E> IntoView for Result<T, E>
where
  T: IntoView + 'static,
  E: std::error::Error + 'static,
{
  fn into_view(self, cx: leptos_reactive::Scope) -> crate::View {
    let errors = match use_context::<RwSignal<Errors>>(cx) {
      Some(e) => e,
      None => {
        #[cfg(debug_assertions)]
        warn!(
          "No ErrorBoundary components found! Returning errors will not be \
           handled and will silently disappear"
        );
        return Transparent::new(()).into_view(cx);
      }
    };

    match self {
      Ok(stuff) => Transparent::new(stuff).into_view(cx),
      Err(error) => {
        errors.update(|errors: &mut Errors| {
          errors.insert(HydrationCtx::id(), error)
        });
        Transparent::new(()).into_view(cx)
      }
    }
  }
}

impl Errors {
  /// Add an error to Errors that will be processed by `<ErrorBoundary/>`
  pub fn insert<E>(&mut self, key: HydrationKey, error: E)
  where
    E: Error + 'static,
  {
    self.0.insert(key, Rc::new(error));
  }
}
