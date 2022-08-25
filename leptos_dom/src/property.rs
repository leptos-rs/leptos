use leptos_reactive::Scope;
use wasm_bindgen::JsValue;

pub enum Property {
    Value(JsValue),
    Fn(Box<dyn Fn() -> JsValue>),
}

pub trait IntoProperty {
    fn into_property(self, cx: Scope) -> Property;
}

impl<T, U> IntoProperty for T
where
    T: Fn() -> U + 'static,
    U: Into<JsValue>,
{
    fn into_property(self, _cx: Scope) -> Property {
        let modified_fn = Box::new(move || self().into());
        Property::Fn(modified_fn)
    }
}

macro_rules! prop_type {
    ($prop_type:ty) => {
        impl IntoProperty for $prop_type {
            fn into_property(self, _cx: Scope) -> Property {
                Property::Value(self.into())
            }
        }

        impl IntoProperty for Option<$prop_type> {
            fn into_property(self, _cx: Scope) -> Property {
                Property::Value(self.into())
            }
        }
    };
}

prop_type!(String);
prop_type!(&String);
prop_type!(&str);
prop_type!(usize);
prop_type!(u8);
prop_type!(u16);
prop_type!(u32);
prop_type!(u64);
prop_type!(u128);
prop_type!(isize);
prop_type!(i8);
prop_type!(i16);
prop_type!(i32);
prop_type!(i64);
prop_type!(i128);
prop_type!(f32);
prop_type!(f64);
prop_type!(bool);
