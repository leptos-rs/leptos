use crate::{HydrationCtx, IntoView};
use cfg_if::cfg_if;
use leptos_reactive::{use_context, RwSignal};
use std::{collections::HashMap, error::Error, sync::Arc};

/// A struct to hold all the possible errors that could be provided by child Views
#[derive(Debug, Clone, Default)]
pub struct Errors(pub HashMap<String, Arc<dyn Error + Send + Sync>>);

impl<T, E> IntoView for Result<T, E>
where
  T: IntoView + 'static,
  E: Error + Send + Sync + 'static,
{
  fn into_view(self, cx: leptos_reactive::Scope) -> crate::View {
    let id = HydrationCtx::peek().previous;
    let errors = use_context::<RwSignal<Errors>>(cx);
    match self {
      Ok(stuff) => {
        if let Some(errors) = errors {
          errors.update(|errors| {
            errors.0.remove(&id);
          });
        }
        stuff.into_view(cx)
      }
      Err(error) => {
        match errors {
          Some(errors) => {
            errors.update({
              #[cfg(all(target_arch = "wasm32", feature = "web"))]
              let id = id.clone();
              move |errors: &mut Errors| errors.insert(id, error)
            });

            // remove the error from the list if this drops,
            // i.e., if it's in a DynChild that switches from Err to Ok
            // Only can run on the client, will panic on the server
            cfg_if! {
              if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
                use leptos_reactive::{on_cleanup, queue_microtask};
                on_cleanup(cx, move || {
                  queue_microtask(move || {
                    errors.update(|errors: &mut Errors| {
                      errors.remove::<E>(&id);
                    });
                  });
                });
              }
            }
          }
          None => {
            #[cfg(debug_assertions)]
            warn!(
              "No ErrorBoundary components found! Returning errors will not \
               be handled and will silently disappear"
            );
          }
        }
        ().into_view(cx)
      }
    }
  }
}
impl Errors {
  /// Add an error to Errors that will be processed by `<ErrorBoundary/>`
  pub fn insert<E>(&mut self, key: String, error: E)
  where
    E: Error + Send + Sync + 'static,
  {
    self.0.insert(key, Arc::new(error));
  }
  /// Add an error with the default key for errors outside the reactive system
  pub fn insert_with_default_key<E>(&mut self, error: E)
  where
    E: Error + Send + Sync + 'static,
  {
    self.0.insert(String::new(), Arc::new(error));
  }
  /// Remove an error to Errors that will be processed by `<ErrorBoundary/>`
  pub fn remove<E>(&mut self, key: &str)
  where
    E: Error + Send + Sync + 'static,
  {
    self.0.remove(key);
  }
}
