use leptos_reactive::Scope;
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
    String(Cow<'static, str>),
    /// A (presumably reactive) function, which will be run inside an effect to do targeted updates to the attribute.
    Fn(Scope, Rc<dyn Fn() -> Attribute>),
    /// An optional string value, which sets the attribute to the value if `Some` and removes the attribute if `None`.
    Option(Scope, Option<Cow<'static, str>>),
    /// A boolean attribute, which sets the attribute if `true` and removes the attribute if `false`.
    Bool(bool),
}

impl Attribute {
    /// Converts the attribute to its HTML value at that moment, including the attribute name,
    /// so it can be rendered on the server.
    pub fn as_value_string(
        &self,
        attr_name: &'static str,
    ) -> Cow<'static, str> {
        match self {
            Attribute::String(value) => {
                format!("{attr_name}=\"{value}\"").into()
            }
            Attribute::Fn(_, f) => {
                let mut value = f();
                while let Attribute::Fn(_, f) = value {
                    value = f();
                }
                value.as_value_string(attr_name)
            }
            Attribute::Option(_, value) => value
                .as_ref()
                .map(|value| format!("{attr_name}=\"{value}\"").into())
                .unwrap_or_default(),
            Attribute::Bool(include) => {
                Cow::Borrowed(if *include { attr_name } else { "" })
            }
        }
    }

    /// Converts the attribute to its HTML value at that moment, not including
    /// the attribute name, so it can be rendered on the server.
    pub fn as_nameless_value_string(&self) -> Option<Cow<'static, str>> {
        match self {
            Attribute::String(value) => Some(value.clone()),
            Attribute::Fn(_, f) => {
                let mut value = f();
                while let Attribute::Fn(_, f) = value {
                    value = f();
                }
                value.as_nameless_value_string()
            }
            Attribute::Option(_, value) => value.as_ref().cloned(),
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
            (Self::Fn(_, _), Self::Fn(_, _)) => false,
            (Self::Option(_, l0), Self::Option(_, r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl std::fmt::Debug for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Fn(_, _) => f.debug_tuple("Fn").finish(),
            Self::Option(_, arg0) => {
                f.debug_tuple("Option").field(arg0).finish()
            }
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
        }
    }
}

/// Converts some type into an [Attribute].
///
/// This is implemented by default for Rust primitive and string types.
pub trait IntoAttribute {
    /// Converts the object into an [Attribute].
    fn into_attribute(self, cx: Scope) -> Attribute;
    /// Helper function for dealing with `Box<dyn IntoAttribute>`.
    fn into_attribute_boxed(self: Box<Self>, cx: Scope) -> Attribute;
}

impl<T: IntoAttribute + 'static> From<T> for Box<dyn IntoAttribute> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl IntoAttribute for Attribute {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        self
    }

    #[inline(always)]
    fn into_attribute_boxed(self: Box<Self>, _: Scope) -> Attribute {
        *self
    }
}

macro_rules! impl_into_attr_boxed {
    () => {
        #[inline(always)]
        fn into_attribute_boxed(self: Box<Self>, cx: Scope) -> Attribute {
            self.into_attribute(cx)
        }
    };
}

impl IntoAttribute for Option<Attribute> {
    #[inline(always)]
    fn into_attribute(self, cx: Scope) -> Attribute {
        self.unwrap_or(Attribute::Option(cx, None))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for String {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        Attribute::String(Cow::Owned(self))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for &'static str {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        Attribute::String(Cow::Borrowed(self))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Cow<'static, str> {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        Attribute::String(self)
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for bool {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        Attribute::Bool(self)
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Option<String> {
    #[inline(always)]
    fn into_attribute(self, cx: Scope) -> Attribute {
        Attribute::Option(cx, self.map(Cow::Owned))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Option<&'static str> {
    #[inline(always)]
    fn into_attribute(self, cx: Scope) -> Attribute {
        Attribute::Option(cx, self.map(Cow::Borrowed))
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for Option<Cow<'static, str>> {
    #[inline(always)]
    fn into_attribute(self, cx: Scope) -> Attribute {
        Attribute::Option(cx, self)
    }

    impl_into_attr_boxed! {}
}

impl<T, U> IntoAttribute for T
where
    T: Fn() -> U + 'static,
    U: IntoAttribute,
{
    fn into_attribute(self, cx: Scope) -> Attribute {
        let modified_fn = Rc::new(move || (self)().into_attribute(cx));
        Attribute::Fn(cx, modified_fn)
    }

    impl_into_attr_boxed! {}
}

impl<T: IntoAttribute> IntoAttribute for (Scope, T) {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        self.1.into_attribute(self.0)
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for (Scope, Option<Box<dyn IntoAttribute>>) {
    fn into_attribute(self, _: Scope) -> Attribute {
        match self.1 {
            Some(bx) => bx.into_attribute_boxed(self.0),
            None => Attribute::Option(self.0, None),
        }
    }

    impl_into_attr_boxed! {}
}

impl IntoAttribute for (Scope, Box<dyn IntoAttribute>) {
    #[inline(always)]
    fn into_attribute(self, _: Scope) -> Attribute {
        self.1.into_attribute_boxed(self.0)
    }

    impl_into_attr_boxed! {}
}

macro_rules! attr_type {
    ($attr_type:ty) => {
        impl IntoAttribute for $attr_type {
            fn into_attribute(self, _: Scope) -> Attribute {
                Attribute::String(self.to_string().into())
            }

            #[inline]
            fn into_attribute_boxed(self: Box<Self>, cx: Scope) -> Attribute {
                self.into_attribute(cx)
            }
        }

        impl IntoAttribute for Option<$attr_type> {
            fn into_attribute(self, cx: Scope) -> Attribute {
                Attribute::Option(cx, self.map(|n| n.to_string().into()))
            }

            #[inline]
            fn into_attribute_boxed(self: Box<Self>, cx: Scope) -> Attribute {
                self.into_attribute(cx)
            }
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
#[inline(never)]
pub fn attribute_helper(
    el: &web_sys::Element,
    name: Cow<'static, str>,
    value: Attribute,
) {
    use leptos_reactive::create_render_effect;
    match value {
        Attribute::Fn(cx, f) => {
            let el = el.clone();
            create_render_effect(cx, move |old| {
                let new = f();
                if old.as_ref() != Some(&new) {
                    attribute_expression(&el, &name, new.clone(), true);
                }
                new
            });
        }
        _ => attribute_expression(el, &name, value, false),
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn attribute_expression(
    el: &web_sys::Element,
    attr_name: &str,
    value: Attribute,
    force: bool,
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
            Attribute::Option(_, value) => {
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
            _ => panic!("Remove nested Fn in Attribute"),
        }
    }
}
