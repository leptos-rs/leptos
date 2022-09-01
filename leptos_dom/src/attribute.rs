use std::rc::Rc;

use leptos_reactive::Scope;

#[derive(Clone)]
pub enum Attribute {
    String(String),
    Fn(Rc<dyn Fn() -> Attribute>),
    Option(Option<String>),
    Bool(bool),
}

impl Attribute {
    pub fn as_value_string(&self, attr_name: &'static str) -> String {
        match self {
            Attribute::String(value) => format!("{attr_name}=\"{value}\""),
            Attribute::Fn(f) => {
                let mut value = f();
                while let Attribute::Fn(f) = value {
                    value = f();
                }
                value.as_value_string(attr_name)
            }
            Attribute::Option(value) => value
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
}

impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Fn(_), Self::Fn(_)) => false,
            (Self::Option(l0), Self::Option(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish(),
            Self::Option(arg0) => f.debug_tuple("Option").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
        }
    }
}

pub trait IntoAttribute {
    fn into_attribute(self, cx: Scope) -> Attribute;
}

impl IntoAttribute for String {
    fn into_attribute(self, _cx: Scope) -> Attribute {
        Attribute::String(self)
    }
}

impl IntoAttribute for bool {
    fn into_attribute(self, _cx: Scope) -> Attribute {
        Attribute::Bool(self)
    }
}

impl IntoAttribute for Option<String> {
    fn into_attribute(self, _cx: Scope) -> Attribute {
        Attribute::Option(self)
    }
}

impl<T, U> IntoAttribute for T
where
    T: Fn() -> U + 'static,
    U: IntoAttribute,
{
    fn into_attribute(self, cx: Scope) -> Attribute {
        let modified_fn = Rc::new(move || (self)().into_attribute(cx));
        Attribute::Fn(modified_fn)
    }
}

macro_rules! attr_type {
    ($attr_type:ty) => {
        impl IntoAttribute for $attr_type {
            fn into_attribute(self, _cx: Scope) -> Attribute {
                Attribute::String(self.to_string())
            }
        }

        impl IntoAttribute for Option<$attr_type> {
            fn into_attribute(self, _cx: Scope) -> Attribute {
                Attribute::Option(self.map(|n| n.to_string()))
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
