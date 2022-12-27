use leptos_reactive::{create_rw_signal, RwSignal, Scope};

/// Contains a shared reference to a DOM node creating while using the `view`
/// macro to create your UI.
///
/// ```
/// # use leptos::*;
/// #[component]
/// pub fn MyComponent(cx: Scope) -> Element {
///   let input_ref = NodeRef::new(cx);
///
///   let on_click = move |_| {
///     let node = input_ref
///       .get()
///       .expect("input_ref should be loaded by now")
///       .unchecked_into::<web_sys::HtmlInputElement>();
///     log!("value is {:?}", node.value())
///   };
///
///   view! {
///     cx,
///     <div>
///     // `node_ref` loads the input
///     <input _ref=input_ref type="text"/>
///     // the button consumes it
///     <button on:click=on_click>"Click me"</button>
///     </div>
///   }
/// }
/// ```
#[derive(Clone, PartialEq)]
pub struct NodeRef<T: Clone + 'static>(RwSignal<Option<T>>);

impl<T: Clone + 'static> NodeRef<T> {
  /// Creates an empty reference.
  pub fn new(cx: Scope) -> Self {
    Self(create_rw_signal(cx, None))
  }

  /// Gets the element that is currently stored in the reference.
  ///
  /// This tracks reactively, so that node references can be used in effects.
  /// Initially, the value will be `None`, but once it is loaded the effect
  /// will rerun and its value will be `Some(Element)`.
  #[track_caller]
  pub fn get(&self) -> Option<T> {
    self.0.get()
  }

  #[doc(hidden)]
  /// Loads an element into the reference. This tracks reactively,
  /// so that effects that use the node reference will rerun once it is loaded,
  /// i.e., effects can be forward-declared.
  #[track_caller]
  pub fn load(&self, node: &T) {
    self.0.update(|current| {
      if current.is_some() {
        crate::debug_warn!(
          "You are setting a NodeRef that has already been filled. It’s \
           possible this is intentional, but it’s also possible that you’re \
           accidentally using the same NodeRef for multiple _ref attributes."
        );
      }
      *current = Some(node.clone());
    });
  }
}

impl<T: Clone + 'static> Copy for NodeRef<T> {}

cfg_if::cfg_if! {
    if #[cfg(not(feature = "stable"))] {
        impl<T: Clone + 'static> FnOnce<()> for NodeRef<T> {
            type Output = Option<T>;

            extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
                self.get()
            }
        }

        impl<T: Clone + 'static> FnMut<()> for NodeRef<T> {
            extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
                self.get()
            }
        }

        impl<T: Clone + 'static> Fn<()> for NodeRef<T> {
            extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
                self.get()
            }
        }
    }
}
