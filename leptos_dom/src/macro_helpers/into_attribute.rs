use std::rc::Rc;

use leptos_reactive::Scope;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::UnwrapThrowExt;

/// Represents the different possible values an attribute node could have.
///
/// This mostly exists for the [`view`](https://docs.rs/leptos_macro/latest/leptos_macro/macro.view.html)
/// macro’s use. You usually won't need to interact with it directly.
#[derive(Clone)]
pub enum Attribute {
  /// A plain string value.
  String(String),
  /// A (presumably reactive) function, which will be run inside an effect to do targeted updates to the attribute.
  Fn(Scope, Rc<dyn Fn() -> Attribute>),
  /// An optional string value, which sets the attribute to the value if `Some` and removes the attribute if `None`.
  Option(Scope, Option<String>),
  /// A boolean attribute, which sets the attribute if `true` and removes the attribute if `false`.
  Bool(bool),
}

impl Attribute {
  /// Converts the attribute to its HTML value at that moment, including the attribute name,
  /// so it can be rendered on the server.
  pub fn as_value_string(&self, attr_name: &'static str) -> String {
    match self {
      Attribute::String(value) => format!("{attr_name}=\"{value}\""),
      Attribute::Fn(_, f) => {
        let mut value = f();
        while let Attribute::Fn(_, f) = value {
          value = f();
        }
        value.as_value_string(attr_name)
      }
      Attribute::Option(_, value) => value
        .as_ref()
        .map(|value| format!("{attr_name}=\"{value}\""))
        .unwrap_or_default(),
      Attribute::Bool(include) => {
        if *include {
          attr_name.to_string()
        } else {
          String::new()
        }
      }
    }
  }

  /// Converts the attribute to its HTML value at that moment, not including
  /// the attribute name, so it can be rendered on the server.
  pub fn as_nameless_value_string(&self) -> String {
    match self {
      Attribute::String(value) => value.to_string(),
      Attribute::Fn(_, f) => {
        let mut value = f();
        while let Attribute::Fn(_, f) = value {
          value = f();
        }
        value.as_nameless_value_string()
      }
      Attribute::Option(_, value) => value
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_default(),
      Attribute::Bool(_) => String::new(),
    }
  }
}

impl PartialEq for Attribute {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::String(l0), Self::String(r0)) => l0 == r0,
      (Self::Fn(_, _), Self::Fn(_, _)) => false,
      (Self::Option(_, l0), Self::Option(_, r0)) => l0 == r0,
      (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
      _ => false,
    }
  }
}

impl std::fmt::Debug for Attribute {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
      Self::Fn(_, _) => f.debug_tuple("Fn").finish(),
      Self::Option(_, arg0) => f.debug_tuple("Option").field(arg0).finish(),
      Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
    }
  }
}

/// Converts some type into an [Attribute].
///
/// This is implemented by default for Rust primitive and string types.
pub trait IntoAttribute {
  /// Converts the object into an [Attribute].
  fn into_attribute(self, cx: Scope) -> Attribute;
}

impl IntoAttribute for String {
  fn into_attribute(self, _: Scope) -> Attribute {
    Attribute::String(self)
  }
}

impl IntoAttribute for bool {
  fn into_attribute(self, _: Scope) -> Attribute {
    Attribute::Bool(self)
  }
}

impl IntoAttribute for Option<String> {
  fn into_attribute(self, cx: Scope) -> Attribute {
    Attribute::Option(cx, self)
  }
}

impl<T, U> IntoAttribute for T
where
  T: Fn() -> U + 'static,
  U: IntoAttribute,
{
  fn into_attribute(self, cx: Scope) -> Attribute {
    let modified_fn = Rc::new(move || (self)().into_attribute(cx));
    Attribute::Fn(cx, modified_fn)
  }
}

impl<T: IntoAttribute> IntoAttribute for (Scope, T) {
  fn into_attribute(self, _: Scope) -> Attribute {
    self.1.into_attribute(self.0)
  }
}

macro_rules! attr_type {
  ($attr_type:ty) => {
    impl IntoAttribute for $attr_type {
      fn into_attribute(self, _: Scope) -> Attribute {
        Attribute::String(self.to_string())
      }
    }

    impl IntoAttribute for Option<$attr_type> {
      fn into_attribute(self, cx: Scope) -> Attribute {
        Attribute::Option(cx, self.map(|n| n.to_string()))
      }
    }
  };
}

attr_type!(&String);
attr_type!(&str);
attr_type!(usize);
attr_type!(u8);
attr_type!(u16);
attr_type!(u32);
attr_type!(u64);
attr_type!(u128);
attr_type!(isize);
attr_type!(i8);
attr_type!(i16);
attr_type!(i32);
attr_type!(i64);
attr_type!(i128);
attr_type!(f32);
attr_type!(f64);
attr_type!(char);

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn attribute_expression(
  el: &web_sys::Element,
  attr_name: &str,
  value: Attribute,
) {
  match value {
    Attribute::String(value) => {
      let value = wasm_bindgen::intern(&value);
      if attr_name == "inner_html" {
        el.set_inner_html(value);
      } else {
        let attr_name = wasm_bindgen::intern(attr_name);
        el.set_attribute(attr_name, value).unwrap_throw();
      }
    }
    Attribute::Option(_, value) => {
      if attr_name == "inner_html" {
        el.set_inner_html(&value.unwrap_or_default());
      } else {
        let attr_name = wasm_bindgen::intern(attr_name);
        match value {
          Some(value) => {
            let value = wasm_bindgen::intern(&value);
            el.set_attribute(attr_name, value).unwrap_throw();
          }
          None => el.remove_attribute(attr_name).unwrap_throw(),
        }
      }
    }
    Attribute::Bool(value) => {
      let attr_name = wasm_bindgen::intern(attr_name);
      if value {
        el.set_attribute(attr_name, attr_name).unwrap_throw();
      } else {
        el.remove_attribute(attr_name).unwrap_throw();
      }
    }
    _ => panic!("Remove nested Fn in Attribute"),
  }
}
