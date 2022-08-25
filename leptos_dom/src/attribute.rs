use leptos_reactive::Scope;

pub enum Attribute {
    String(String),
    Fn(Box<dyn Fn() -> Attribute>),
    Option(Option<String>),
    Bool(bool),
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
        let modified_fn = Box::new(move || (self)().into_attribute(cx));
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
