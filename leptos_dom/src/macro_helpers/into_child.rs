use std::{cell::{OnceCell, RefCell}, hash::Hash, rc::Rc};
use cfg_if::cfg_if;
use leptos_reactive::{Scope, create_effect};
use crate::{IntoNode, ComponentRepr, EachKey, Node, HtmlElement, Text, Element, Fragment, Unit, text, DynChild, IntoElement, Component};

pub enum Child {
    /// A (presumably reactive) function, which will be run inside an effect to do targeted updates to the node.
    Fn(Box<RefCell<dyn FnMut() -> Child>>),
	/// Content for a text node.
	Text(String),
    /// A generic node (a text node, comment, or element.)
    Node(Node),
	/// Nothing
	Unit
}

impl IntoNode for Child {
    fn into_node(self, cx: Scope) -> Node {
        match self {
			Child::Node(node) => node,
			Child::Unit => Unit.into_node(cx),
			Child::Text(data) => text(data),
			Child::Fn(f) => DynChild::new(move || {
				let mut value = (f.borrow_mut())();
				while let Child::Fn(f) = value {
					value = (f.borrow_mut())();
				}
				value.into_node(cx)
			}).into_node(cx)
		}
    }
}

pub trait IntoChild {
	fn into_child(self, cx: Scope) -> Child;
}

impl IntoChild for Node {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Node(self)
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

impl<IF, I, T, EF, N, KF, K> IntoChild for EachKey<IF, I, T, EF, N, KF, K>
where
  IF: Fn() -> I + 'static,
  I: IntoIterator<Item = T>,
  EF: Fn(T) -> N + 'static,
  N: IntoNode,
  KF: Fn(&T) -> K + 'static,
  K: Eq + Hash + 'static,
  T: 'static,
  {
    fn into_child(self, cx: Scope) -> Child {
        Child::Node(self.into_node(cx))
    }
}

impl<T, U> IntoChild for T
where
    T: FnMut() -> U + 'static,
    U: IntoChild,
{
    fn into_child(mut self, cx: Scope) -> Child {
        let modified_fn = Box::new(RefCell::new(move || (self)().into_child(cx)));
        Child::Fn(modified_fn)
    }
}

macro_rules! node_type {
    ($child_type:ty) => {
        impl IntoChild for $child_type {
            fn into_child(self, cx: Scope) -> Child {
                Child::Node(self.into_node(cx))
            }
        }
    };
}

node_type!(());
node_type!(Element);
node_type!(Text);
node_type!(Vec<Node>);
node_type!(Fragment);
node_type!(ComponentRepr);


impl<El: IntoElement> IntoChild for HtmlElement<El> {
    fn into_child(self, cx: Scope) -> Child {
        Child::Node(self.into_node(cx))
    }
}

impl<F> IntoChild for Component<F>
where
  F: FnOnce(Scope) -> Node,
{
    fn into_child(self, cx: Scope) -> Child {
        Child::Node(self.into_node(cx))
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