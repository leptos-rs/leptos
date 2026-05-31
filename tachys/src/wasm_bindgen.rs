#[derive(Clone, Default)]
pub struct JsValue;

impl JsValue {
    pub const UNDEFINED: JsValue = JsValue;
    pub fn null() -> Self {
        Self
    }
    pub fn from_str(s: &str) -> Self {
        let _ = s;
        Self
    }
}

pub mod closure {
    use crate::wasm_bindgen::JsValue;
    pub struct Closure<T: ?Sized>(std::marker::PhantomData<T>);
    impl<T: ?Sized> Closure<T> {
        pub fn wrap(val: Box<T>) -> Self {
            let _ = val;
            Closure(std::marker::PhantomData)
        }
        pub fn into_js_value(self) -> JsValue {
            JsValue
        }
    }
}

pub mod prelude {
    pub use crate::wasm_bindgen::closure::Closure;
}

pub trait UnwrapThrowExt {
    type Type;
    fn unwrap_throw(self) -> Self::Type;
}

impl<T> UnwrapThrowExt for Option<T> {
    type Type = T;
    fn unwrap_throw(self) -> Self::Type {
        self.unwrap()
    }
}

impl<T, E> UnwrapThrowExt for Result<T, E>
where
    E: std::fmt::Debug,
{
    type Type = T;
    fn unwrap_throw(self) -> Self::Type {
        self.unwrap()
    }
}

pub trait JsCast {
    fn dyn_into<T>(self) -> Result<T, Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }
    fn dyn_ref<T>(&self) -> Option<&T> {
        None
    }
    fn unchecked_into<T>(self) -> T
    where
        Self: Sized,
    {
        unimplemented!()
    }
    fn unchecked_ref<T>(&self) -> &T {
        unimplemented!()
    }
}

impl JsCast for () {}
impl JsCast for JsValue {}

// From implementations to support standard conversions
macro_rules! impl_from {
    ($($t:ty),* $(,)?) => {
        $(
            impl From<$t> for JsValue {
                fn from(_: $t) -> Self {
                    JsValue
                }
            }
            impl From<Option<$t>> for JsValue {
                fn from(_: Option<$t>) -> Self {
                    JsValue
                }
            }
        )*
    };
}

impl_from![
    bool,
    String,
    f64,
    f32,
    i8,
    u8,
    i16,
    u16,
    i32,
    u32,
    i64,
    u64,
    i128,
    u128,
    isize,
    usize,
];

impl From<Option<JsValue>> for JsValue {
    fn from(_: Option<JsValue>) -> Self {
        JsValue
    }
}

impl<'a> From<&'a str> for JsValue {
    fn from(_: &'a str) -> Self {
        JsValue
    }
}

impl<'a> From<Option<&'a str>> for JsValue {
    fn from(_: Option<&'a str>) -> Self {
        JsValue
    }
}

impl<'a, 'b> From<std::borrow::Cow<'a, str>> for JsValue {
    fn from(_: std::borrow::Cow<'a, str>) -> Self {
        JsValue
    }
}

impl<'a> From<&'a String> for JsValue {
    fn from(_: &'a String) -> Self {
        JsValue
    }
}

impl<'a> From<Option<&'a String>> for JsValue {
    fn from(_: Option<&'a String>) -> Self {
        JsValue
    }
}
