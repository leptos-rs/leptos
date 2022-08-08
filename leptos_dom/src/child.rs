use leptos_reactive::{ReadSignal, Scope};
use wasm_bindgen::JsCast;

type Node = web_sys::Node;

#[derive(Clone)]
pub enum Child<'a> {
    Null,
    Text(String),
    Fn(&'a dyn Fn() -> Child<'a>),
    Node(Node),
    Nodes(Vec<Node>),
}

impl<'a> std::fmt::Debug for Child<'a> {
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

impl<'a> PartialEq for Child<'a> {
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

pub trait IntoChild<'a> {
    fn into_child(self, cx: Scope<'a>) -> Child<'a>;
}

impl<'a> IntoChild<'a> for Child<'a> {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        self
    }
}

impl<'a> IntoChild<'a> for String {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Text(self)
    }
}

impl<'a> IntoChild<'a> for web_sys::Node {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Node(self)
    }
}

impl<'a> IntoChild<'a> for web_sys::Text {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Node(self.unchecked_into())
    }
}

impl<'a> IntoChild<'a> for web_sys::Element {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Node(self.unchecked_into())
    }
}

impl<'a, T> IntoChild<'a> for Option<T>
where
    T: IntoChild<'a>,
{
    fn into_child(self, cx: Scope<'a>) -> Child<'a> {
        match self {
            Some(val) => val.into_child(cx),
            None => Child::Null,
        }
    }
}

impl<'a> IntoChild<'a> for Vec<web_sys::Node> {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Nodes(self)
    }
}

impl<'a> IntoChild<'a> for Vec<web_sys::Element> {
    fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
        Child::Nodes(
            self.into_iter()
                .map(|el| el.unchecked_into::<web_sys::Node>())
                .collect(),
        )
    }
}

impl<'a, T, U> IntoChild<'a> for T
where
    T: Fn() -> U + 'a,
    U: IntoChild<'a>,
{
    fn into_child(self, cx: Scope<'a>) -> Child<'a> {
        let modified_fn = cx.create_ref(move || (self)().into_child(cx));
        Child::Fn(modified_fn)
    }
}

macro_rules! child_type {
    ($child_type:ty) => {
        impl<'a> IntoChild<'a> for $child_type {
            fn into_child(self, _cx: Scope<'a>) -> Child<'a> {
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
