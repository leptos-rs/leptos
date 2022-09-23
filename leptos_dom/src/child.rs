use std::{cell::RefCell, rc::Rc};

use leptos_reactive::Scope;

#[cfg(any(feature = "csr", feature = "hydrate"))]
use wasm_bindgen::JsCast;

use crate::Node;

#[derive(Clone)]
pub enum Child {
    Null,
    Text(String),
    Fn(Rc<RefCell<dyn FnMut() -> Child>>),
    Node(Node),
    Nodes(Vec<Node>),
}

impl Child {
    #[cfg(feature = "ssr")]
    pub fn as_child_string(&self) -> String {
        match self {
            Child::Null => String::new(),
            Child::Text(text) => text.to_string(),
            Child::Fn(f) => {
                let mut value = (f.borrow_mut())();
                while let Child::Fn(f) = value {
                    value = (f.borrow_mut())();
                }
                value.as_child_string()
            }
            Child::Node(node) => node.to_string(),
            Child::Nodes(nodes) => nodes.iter().cloned().collect(),
        }
    }
}

impl std::fmt::Debug for Child {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "Null"),
            Self::Text(arg0) => f.debug_tuple("Text").field(arg0).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish(),
            Self::Node(arg0) => f.debug_tuple("Node").field(arg0).finish(),
            Self::Nodes(arg0) => f.debug_tuple("Nodes").field(arg0).finish(),
        }
    }
}

impl PartialEq for Child {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(l0), Self::Text(r0)) => l0 == r0,
            (Self::Fn(l0), Self::Fn(r0)) => std::ptr::eq(l0, r0),
            (Self::Node(l0), Self::Node(r0)) => l0 == r0,
            (Self::Nodes(l0), Self::Nodes(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

pub trait IntoChild {
    fn into_child(self, cx: Scope) -> Child;
}

impl IntoChild for Child {
    fn into_child(self, _cx: Scope) -> Child {
        self
    }
}

impl IntoChild for () {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Null
    }
}

impl IntoChild for String {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Text(self)
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl IntoChild for web_sys::Node {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Node(self)
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl IntoChild for web_sys::Text {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Node(self.unchecked_into())
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl IntoChild for web_sys::Element {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Node(self.unchecked_into())
    }
}

impl<T> IntoChild for Option<T>
where
    T: IntoChild,
{
    fn into_child(self, cx: Scope) -> Child {
        match self {
            Some(val) => val.into_child(cx),
            None => Child::Null,
        }
    }
}

impl IntoChild for Vec<Node> {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Nodes(self)
    }
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl IntoChild for Vec<web_sys::Element> {
    fn into_child(self, _cx: Scope) -> Child {
        Child::Nodes(
            self.into_iter()
                .map(|el| el.unchecked_into::<web_sys::Node>())
                .collect(),
        )
    }
}

impl<T, U> IntoChild for T
where
    T: FnMut() -> U + 'static,
    U: IntoChild,
{
    fn into_child(mut self, cx: Scope) -> Child {
        let modified_fn = Rc::new(RefCell::new(move || (self)().into_child(cx)));
        Child::Fn(modified_fn)
    }
}

macro_rules! child_type {
    ($child_type:ty) => {
        impl IntoChild for $child_type {
            fn into_child(self, _cx: Scope) -> Child {
                Child::Text(self.to_string())
            }
        }
    };
}

child_type!(&String);
child_type!(&str);
child_type!(usize);
child_type!(u8);
child_type!(u16);
child_type!(u32);
child_type!(u64);
child_type!(u128);
child_type!(isize);
child_type!(i8);
child_type!(i16);
child_type!(i32);
child_type!(i64);
child_type!(i128);
child_type!(f32);
child_type!(f64);
child_type!(char);
child_type!(bool);
