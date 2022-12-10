use crate::{
  Component, ComponentRepr, DynChild, Each, Element, Fragment, HtmlElement,
  IntoElement, IntoView, Text, Unit, View,
};
use leptos_reactive::Scope;
use std::{cell::RefCell, hash::Hash};

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
  EF: Fn(T) -> N + 'static,
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
  fn into_child(self, _: Scope) -> Child {
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

impl<F, V> IntoChild for Component<F, V>
where
  F: FnOnce(Scope) -> V,
  V: IntoView,
{
  fn into_child(self, cx: Scope) -> Child {
    Child::View(self.into_view(cx))
  }
}

macro_rules! text_type {
  ($child_type:ty) => {
    impl IntoChild for $child_type {
      fn into_child(self, _: Scope) -> Child {
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

macro_rules! impl_into_child_for_tuples {
  ($($ty:ident),* $(,)?) => {
    impl<$($ty),*> IntoChild for ($($ty,)*)
    where
      $($ty: IntoView),*
    {
      fn into_child(self, cx: Scope) -> Child {
        paste::paste! {
          self.into_view(cx).into_child(cx)
        }
      }
    }
  };
}

impl_into_child_for_tuples!(A);
impl_into_child_for_tuples!(A, B);
impl_into_child_for_tuples!(A, B, C);
impl_into_child_for_tuples!(A, B, C, D);
impl_into_child_for_tuples!(A, B, C, D, E);
impl_into_child_for_tuples!(A, B, C, D, E, F);
impl_into_child_for_tuples!(A, B, C, D, E, F, G);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_into_child_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_into_child_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
