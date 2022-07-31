use leptos_reactive::Scope;

pub enum Attribute<'a> {
    String(String),
    Fn(&'a dyn Fn() -> Attribute<'a>),
    Option(Option<String>),
    Bool(bool),
}

pub trait IntoAttribute<'a> {
    fn into_attribute(self, cx: Scope<'a>) -> Attribute<'a>;
}

impl<'a> IntoAttribute<'a> for String {
    fn into_attribute(self, _cx: Scope<'a>) -> Attribute<'a> {
        Attribute::String(self)
    }
}

impl<'a> IntoAttribute<'a> for bool {
    fn into_attribute(self, _cx: Scope<'a>) -> Attribute<'a> {
        Attribute::Bool(self)
    }
}

impl<'a> IntoAttribute<'a> for Option<String> {
    fn into_attribute(self, _cx: Scope<'a>) -> Attribute<'a> {
        Attribute::Option(self)
    }
}

impl<'a, T, U> IntoAttribute<'a> for T
where
    T: Fn() -> U + 'a,
    U: IntoAttribute<'a>,
{
    fn into_attribute(self, cx: Scope<'a>) -> Attribute<'a> {
        let modified_fn = cx.create_ref(move || (self)().into_attribute(cx));
        Attribute::Fn(modified_fn)
    }
}

macro_rules! attr_type {
    ($attr_type:ty) => {
        impl<'a> IntoAttribute<'a> for $attr_type {
            fn into_attribute(self, _cx: Scope<'a>) -> Attribute<'a> {
                Attribute::String(self.to_string())
            }
        }

        impl<'a> IntoAttribute<'a> for Option<$attr_type> {
            fn into_attribute(self, _cx: Scope<'a>) -> Attribute<'a> {
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
