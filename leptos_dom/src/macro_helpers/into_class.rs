use leptos_reactive::Scope;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::UnwrapThrowExt;

/// Represents the different possible values a single class on an element could have,
/// allowing you to do fine-grained updates to single items
/// in [`Element.classList`](https://developer.mozilla.org/en-US/docs/Web/API/Element/classList).
///
/// This mostly exists for the [`view`](https://docs.rs/leptos_macro/latest/leptos_macro/macro.view.html)
/// macro’s use. You usually won't need to interact with it directly.
pub enum Class {
  /// Whether the class is present.
  Value(bool),
  /// A (presumably reactive) function, which will be run inside an effect to toggle the class.
  Fn(Scope, Box<dyn Fn() -> bool>),
}

/// Converts some type into a [Class].
pub trait IntoClass {
  /// Converts the object into a [Class].
  fn into_class(self, cx: Scope) -> Class;
}

impl IntoClass for bool {
  fn into_class(self, _cx: Scope) -> Class {
    Class::Value(self)
  }
}

impl<T> IntoClass for T
where
  T: Fn() -> bool + 'static,
{
  fn into_class(self, cx: Scope) -> Class {
    let modified_fn = Box::new(self);
    Class::Fn(cx, modified_fn)
  }
}

impl Class {
  /// Converts the class to its HTML value at that moment so it can be rendered on the server.
  pub fn as_value_string(&self, class_name: &'static str) -> &'static str {
    match self {
      Class::Value(value) => {
        if *value {
          class_name
        } else {
          ""
        }
      }
      Class::Fn(_, f) => {
        let value = f();
        if value {
          class_name
        } else {
          ""
        }
      }
    }
  }
}

impl<T: IntoClass> IntoClass for (Scope, T) {
  fn into_class(self, _: Scope) -> Class {
    self.1.into_class(self.0)
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn class_expression(
  class_list: &web_sys::DomTokenList,
  class_name: &str,
  value: bool,
) {
  let class_name = wasm_bindgen::intern(class_name);
  if value {
    class_list.add_1(class_name).unwrap_throw();
  } else {
    class_list.remove_1(class_name).unwrap_throw();
  }
}
