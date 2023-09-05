#[cfg(not(feature = "nightly"))]
use leptos_reactive::{
    MaybeProp, MaybeSignal, Memo, ReadSignal, RwSignal, Signal, SignalGet,
};
use wasm_bindgen::JsValue;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::UnwrapThrowExt;

/// Represents the different possible values an element property could have,
/// allowing you to do fine-grained updates to single fields.
///
/// This mostly exists for the [`view`](https://docs.rs/leptos_macro/latest/leptos_macro/macro.view.html)
/// macro’s use. You usually won't need to interact with it directly, but it can be useful for defining
/// permissive APIs for certain components.
pub enum Property {
    /// A static JavaScript value.
    Value(JsValue),
    /// A (presumably reactive) function, which will be run inside an effect to update the property.
    Fn(Box<dyn Fn() -> JsValue>),
}

/// Converts some type into a [`Property`].
///
/// This is implemented by default for Rust primitive types, [`String`] and friends, and [`JsValue`].
pub trait IntoProperty {
    /// Converts the object into a [`Property`].
    fn into_property(self) -> Property;

    /// Helper function for dealing with `Box<dyn IntoProperty>`.
    fn into_property_boxed(self: Box<Self>) -> Property;
}

impl<T, U> IntoProperty for T
where
    T: Fn() -> U + 'static,
    U: Into<JsValue>,
{
    fn into_property(self) -> Property {
        let modified_fn = Box::new(move || self().into());
        Property::Fn(modified_fn)
    }

    fn into_property_boxed(self: Box<Self>) -> Property {
        (*self).into_property()
    }
}

macro_rules! prop_type {
    ($prop_type:ty) => {
        impl IntoProperty for $prop_type {
            #[inline(always)]
            fn into_property(self) -> Property {
                Property::Value(self.into())
            }

            fn into_property_boxed(self: Box<Self>) -> Property {
                (*self).into_property()
            }
        }

        impl IntoProperty for Option<$prop_type> {
            #[inline(always)]
            fn into_property(self) -> Property {
                Property::Value(self.into())
            }

            fn into_property_boxed(self: Box<Self>) -> Property {
                (*self).into_property()
            }
        }
    };
}

macro_rules! prop_signal_type {
    ($signal_type:ty) => {
        #[cfg(not(feature = "nightly"))]
        impl<T> IntoProperty for $signal_type
        where
            T: Into<JsValue> + Clone,
        {
            fn into_property(self) -> Property {
                let modified_fn = Box::new(move || self.get().into());
                Property::Fn(modified_fn)
            }

            fn into_property_boxed(self: Box<Self>) -> Property {
                (*self).into_property()
            }
        }
    };
}

macro_rules! prop_signal_type_optional {
    ($signal_type:ty) => {
        #[cfg(not(feature = "nightly"))]
        impl<T> IntoProperty for $signal_type
        where
            T: Clone,
            Option<T>: Into<JsValue>,
        {
            fn into_property(self) -> Property {
                let modified_fn = Box::new(move || self.get().into());
                Property::Fn(modified_fn)
            }

            fn into_property_boxed(self: Box<Self>) -> Property {
                (*self).into_property()
            }
        }
    };
}

prop_type!(JsValue);
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

prop_signal_type!(ReadSignal<T>);
prop_signal_type!(RwSignal<T>);
prop_signal_type!(Memo<T>);
prop_signal_type!(Signal<T>);
prop_signal_type!(MaybeSignal<T>);
prop_signal_type_optional!(MaybeProp<T>);

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use leptos_reactive::Oco;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn property_helper(
    el: &web_sys::Element,
    name: Oco<'static, str>,
    value: Property,
) {
    use leptos_reactive::create_render_effect;

    match value {
        Property::Fn(f) => {
            let el = el.clone();
            create_render_effect(move |_| {
                let new = f();
                let prop_name = wasm_bindgen::intern(&name);
                property_expression(&el, prop_name, new.clone());
                new
            });
        }
        Property::Value(value) => {
            let prop_name = wasm_bindgen::intern(&name);
            property_expression(el, prop_name, value)
        }
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn property_expression(
    el: &web_sys::Element,
    prop_name: &str,
    value: JsValue,
) {
    js_sys::Reflect::set(el, &JsValue::from_str(prop_name), &value)
        .unwrap_throw();
}
