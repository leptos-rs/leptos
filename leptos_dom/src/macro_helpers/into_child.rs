use crate::{
  Component, ComponentRepr, DynChild, Each, Element, Fragment, HtmlElement,
  IntoElement, IntoView, Text, Unit, View,
};
use cfg_if::cfg_if;
use leptos_reactive::{create_effect, Scope};
use std::{
  cell::{OnceCell, RefCell},
  hash::Hash,
  rc::Rc,
};

pub enum Child {
  /// A (presumably reactive) function, which will be run inside an effect to do targeted updates to the node.
  Fn(Scope, Box<RefCell<dyn FnMut() -> Child>>),
  /// Content for a text node.
  Text(String),
  /// A generic node (a text node, comment, or element.)
  View(View),
  /// Nothing
  Unit,
}

impl IntoView for Child {
  fn into_view(self, cx: Scope) -> View {
    match self {
      Child::View(node) => node,
      Child::Unit => Unit.into_view(cx),
      Child::Text(data) => crate::html::text(data).into_view(cx),
      Child::Fn(cx, f) => DynChild::new(move || {
        let mut value = (f.borrow_mut())();
        let mut cx = cx;
        while let Child::Fn(mapped_cx, f) = value {
          value = (f.borrow_mut())();
          cx = mapped_cx;
        }
        value.into_view(cx)
      })
      .into_view(cx),
    }
  }
}

pub trait IntoChild {
  fn into_child(self, cx: Scope) -> Child;
}

impl IntoChild for View {
  fn into_child(self, _cx: Scope) -> Child {
    Child::View(self)
  }
}

impl IntoChild for String {
  fn into_child(self, _cx: Scope) -> Child {
    Child::Text(self)
  }
}

impl<T> IntoChild for Option<T>
where
  T: IntoChild,
{
  fn into_child(self, cx: Scope) -> Child {
    match self {
      Some(val) => val.into_child(cx),
      None => Child::Unit,
    }
  }
}

impl<IF, I, T, EF, N, KF, K> IntoChild for Each<IF, I, T, EF, N, KF, K>
where
  IF: Fn() -> I + 'static,
  I: IntoIterator<Item = T>,
  EF: Fn(Scope, T) -> N + 'static,
  N: IntoView,
  KF: Fn(&T) -> K + 'static,
  K: Eq + Hash + 'static,
  T: 'static,
{
  fn into_child(self, cx: Scope) -> Child {
    Child::View(self.into_view(cx))
  }
}

impl<T, U> IntoChild for T
where
  T: FnMut() -> U + 'static,
  U: IntoChild,
{
  fn into_child(mut self, cx: Scope) -> Child {
    let modified_fn = Box::new(RefCell::new(move || (self)().into_child(cx)));
    Child::Fn(cx, modified_fn)
  }
}

impl<T: IntoChild> IntoChild for (Scope, T) {
  fn into_child(self, cx: Scope) -> Child {
    self.1.into_child(self.0)
  }
}

macro_rules! node_type {
  ($child_type:ty) => {
    impl IntoChild for $child_type {
      fn into_child(self, cx: Scope) -> Child {
        Child::View(self.into_view(cx))
      }
    }
  };
}

node_type!(());
node_type!(Element);
node_type!(Text);
node_type!(Vec<View>);
node_type!(Fragment);
node_type!(ComponentRepr);

impl<El: IntoElement> IntoChild for HtmlElement<El> {
  fn into_child(self, cx: Scope) -> Child {
    Child::View(self.into_view(cx))
  }
}

impl<F> IntoChild for Component<F>
where
  F: FnOnce(Scope) -> View,
{
  fn into_child(self, cx: Scope) -> Child {
    Child::View(self.into_view(cx))
  }
}

macro_rules! text_type {
  ($child_type:ty) => {
    impl IntoChild for $child_type {
      fn into_child(self, cx: Scope) -> Child {
        Child::Text(self.to_string().into())
      }
    }
  };
}

text_type!(&String);
text_type!(&str);
text_type!(usize);
text_type!(u8);
text_type!(u16);
text_type!(u32);
text_type!(u64);
text_type!(u128);
text_type!(isize);
text_type!(i8);
text_type!(i16);
text_type!(i32);
text_type!(i64);
text_type!(i128);
text_type!(f32);
text_type!(f64);
text_type!(char);
text_type!(bool);
