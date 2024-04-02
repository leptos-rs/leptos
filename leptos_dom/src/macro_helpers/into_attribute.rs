#[cfg(not(feature = "nightly"))]
use leptos_reactive::{
    MaybeProp, MaybeSignal, Memo, ReadSignal, RwSignal, Signal, SignalGet,
};
use leptos_reactive::{Oco, TextProp};
use std::{borrow::Cow, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::UnwrapThrowExt;

/// Represents the different possible values an attribute node could have.
///
/// This mostly exists for the [`view`](https://docs.rs/leptos_macro/latest/leptos_macro/macro.view.html)
/// macro’s use. You usually won't need to interact with it directly, but it can be useful for defining
/// permissive APIs for certain components.
#[derive(Clone)]
pub enum Attribute {
    /// A plain string value.
    String(Oco<'static, str>),
    /// A (presumably reactive) function, which will be run inside an effect to do targeted updates to the attribute.
    Fn(Rc<dyn Fn() -> Attribute>),
    /// An optional string value, which sets the attribute to the value if `Some` and removes the attribute if `None`.
    Option(Option<Oco<'static, str>>),
    /// A boolean attribute, which sets the attribute if `true` and removes the attribute if `false`.
    Bool(bool),
}

impl Attribute {
    /// Converts the attribute to its HTML value at that moment, including the attribute name,
    /// so it can be rendered on the server.
    pub fn as_value_string(
        &self,
        attr_name: &'static str,
    ) -> Oco<'static, str> {
        match self {
            Attribute::String(value) => {
                format!("{attr_name}=\"{value}\"").into()
            }
            Attribute::Fn(f) => {
                let mut value = f();
                while let Attribute::Fn(f) = value {
                    value = f();
                }
                value.as_value_string(attr_name)
            }
            Attribute::Option(value) => value
                .as_ref()
                .map(|value| format!("{attr_name}=\"{value}\"").into())
                .unwrap_or_default(),
            Attribute::Bool(include) => {
                Oco::Borrowed(if *include { attr_name } else { "" })
            }
        }
    }

    /// Converts the attribute to its HTML value at that moment, not including
    /// the attribute name, so it can be rendered on the server.
    pub fn as_nameless_value_string(&self) -> Option<Oco<'static, str>> {
        match self {
            Attribute::String(value) => Some(value.clone()),
            Attribute::Fn(f) => {
                let mut value = f();
                while let Attribute::Fn(f) = value {
                    value = f();
                }
                value.as_nameless_value_string()
            }
            Attribute::Option(value) => value.as_ref().cloned(),
            Attribute::Bool(include) => {
                if *include {
                    Some("".into())
                } else {
                    None
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

impl core::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish(),
            Self::Option(arg0) => f.debug_tuple("Option").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
        }
    }
}

/// Converts some type into an [`Attribute`].
///
/// This is implemented by default for Rust primitive and string types.
pub trait IntoAttribute {
    /// Converts the object into an [`Attribute`].
    fn into_attribute(self) -> Attribute;

    /// Helper function for dealing with `Box<dyn IntoAttribute>`.
    fn into_attribute_boxed(self: Box<Self>) -> Attribute;
}

impl<T: IntoAttribute + 'static> From<T> for Box<dyn IntoAttribute> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl IntoAttribute for Attribute {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        self
    }

    #[inline(always)]
    fn into_attribute_boxed(self: Box<Self>) -> Attribute {
        *self
    }
}

macro_rules! impl_into_attr_boxed {
    () => {
        #[inline(always)]
        fn into_attribute_boxed(self: Box<Self>) -> Attribute {
            self.into_attribute()
        }
    };
}

impl IntoAttribute for String {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::String(Oco::Owned(self))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for &'static str {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::String(Oco::Borrowed(self))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Cow<'static, str> {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::String(self.into())
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Oco<'static, str> {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::String(self)
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Rc<str> {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::String(Oco::Counted(self))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for bool {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        Attribute::Bool(self)
    }

    impl_into_attr_boxed! {}
}

impl<T: IntoAttribute> IntoAttribute for Option<T> {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        self.map_or(Attribute::Option(None), IntoAttribute::into_attribute)
    }

    impl_into_attr_boxed! {}
}

impl<T, U> IntoAttribute for T
where
    T: Fn() -> U + 'static,
    U: IntoAttribute,
{
    fn into_attribute(self) -> Attribute {
        let modified_fn = Rc::new(move || (self)().into_attribute());
        Attribute::Fn(modified_fn)
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Option<Box<dyn IntoAttribute>> {
    fn into_attribute(self) -> Attribute {
        match self {
            Some(bx) => bx.into_attribute_boxed(),
            None => Attribute::Option(None),
        }
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for TextProp {
    fn into_attribute(self) -> Attribute {
        (move || self.get()).into_attribute()
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for core::fmt::Arguments<'_> {
    fn into_attribute(self) -> Attribute {
        match self.as_str() {
            Some(s) => s.into_attribute(),
            None => self.to_string().into_attribute(),
        }
    }

    impl_into_attr_boxed! {}
}

/* impl IntoAttribute for Box<dyn IntoAttribute> {
    #[inline(always)]
    fn into_attribute(self) -> Attribute {
        self.into_attribute_boxed()
    }

    impl_into_attr_boxed! {}
} */

macro_rules! attr_type {
    ($attr_type:ty) => {
        impl IntoAttribute for $attr_type {
            fn into_attribute(self) -> Attribute {
                Attribute::String(self.to_string().into())
            }

            #[inline]
            fn into_attribute_boxed(self: Box<Self>) -> Attribute {
                self.into_attribute()
            }
        }
    };
}

macro_rules! attr_signal_type {
    ($signal_type:ty) => {
        #[cfg(not(feature = "nightly"))]
        impl<T> IntoAttribute for $signal_type
        where
            T: IntoAttribute + Clone,
        {
            fn into_attribute(self) -> Attribute {
                let modified_fn = Rc::new(move || self.get().into_attribute());
                Attribute::Fn(modified_fn)
            }

            impl_into_attr_boxed! {}
        }
    };
}

attr_type!(&String);
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

attr_signal_type!(ReadSignal<T>);
attr_signal_type!(RwSignal<T>);
attr_signal_type!(Memo<T>);
attr_signal_type!(Signal<T>);
attr_signal_type!(MaybeSignal<T>);
attr_signal_type!(MaybeProp<T>);

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
#[inline(never)]
#[track_caller]
pub fn attribute_helper(
    el: &web_sys::Element,
    name: Oco<'static, str>,
    value: Attribute,
) {
    #[cfg(debug_assertions)]
    let called_at = std::panic::Location::caller();
    use leptos_reactive::create_render_effect;
    match value {
        Attribute::Fn(f) => {
            let el = el.clone();
            create_render_effect(move |old| {
                let new = f();
                if old.as_ref() != Some(&new) {
                    attribute_expression(
                        &el,
                        &name,
                        new.clone(),
                        true,
                        #[cfg(debug_assertions)]
                        called_at,
                    );
                }
                new
            });
        }
        _ => attribute_expression(
            el,
            &name,
            value,
            false,
            #[cfg(debug_assertions)]
            called_at,
        ),
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn attribute_expression(
    el: &web_sys::Element,
    attr_name: &str,
    value: Attribute,
    force: bool,
    #[cfg(debug_assertions)] called_at: &'static std::panic::Location<'static>,
) {
    use crate::HydrationCtx;

    if force || !HydrationCtx::is_hydrating() {
        match value {
            Attribute::String(value) => {
                let value = wasm_bindgen::intern(&value);
                if attr_name == "inner_html" {
                    el.set_inner_html(value);
                } else {
                    let attr_name = wasm_bindgen::intern(attr_name);
                    el.set_attribute(attr_name, value).unwrap_throw();
                }
            }
            Attribute::Option(value) => {
                if attr_name == "inner_html" {
                    el.set_inner_html(&value.unwrap_or_default());
                } else {
                    let attr_name = wasm_bindgen::intern(attr_name);
                    match value {
                        Some(value) => {
                            let value = wasm_bindgen::intern(&value);
                            el.set_attribute(attr_name, value).unwrap_throw();
                        }
                        None => el.remove_attribute(attr_name).unwrap_throw(),
                    }
                }
            }
            Attribute::Bool(value) => {
                let attr_name = wasm_bindgen::intern(attr_name);
                if value {
                    el.set_attribute(attr_name, attr_name).unwrap_throw();
                } else {
                    el.remove_attribute(attr_name).unwrap_throw();
                }
            }
            Attribute::Fn(f) => {
                let mut v = f();
                crate::debug_warn!(
                    "At {called_at}, you are providing a dynamic attribute \
                     with a nested function. For example, you might have a \
                     closure that returns another function instead of a \
                     value. This creates some added overhead. If possible, \
                     you should instead provide a function that returns a \
                     value instead.",
                );
                while let Attribute::Fn(f) = v {
                    v = f();
                }
                attribute_expression(
                    el,
                    attr_name,
                    v,
                    force,
                    #[cfg(debug_assertions)]
                    called_at,
                );
            }
        }
    }
}
