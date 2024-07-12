use leptos_reactive::Oco;
#[cfg(not(feature = "nightly"))]
use leptos_reactive::{
    MaybeProp, MaybeSignal, Memo, ReadSignal, RwSignal, Signal, SignalGet,
};
use std::{borrow::Cow, rc::Rc};

/// todo docs
#[derive(Clone)]
pub enum Style {
    /// A plain string value.
    Value(Oco<'static, str>),
    /// An optional string value, which sets the property to the value if `Some` and removes the property if `None`.
    Option(Option<Oco<'static, str>>),
    /// A (presumably reactive) function, which will be run inside an effect to update the style.
    Fn(Rc<dyn Fn() -> Style>),
}

impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Value(l0), Self::Value(r0)) => l0 == r0,
            (Self::Fn(_), Self::Fn(_)) => false,
            (Self::Option(l0), Self::Option(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl core::fmt::Debug for Style {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Value(arg0) => f.debug_tuple("Value").field(arg0).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish(),
            Self::Option(arg0) => f.debug_tuple("Option").field(arg0).finish(),
        }
    }
}

/// Converts some type into a [`Style`].
pub trait IntoStyle {
    /// Converts the object into a [`Style`].
    fn into_style(self) -> Style;

    /// Helper function for dealing with `Box<dyn IntoStyle>`.
    fn into_style_boxed(self: Box<Self>) -> Style;
}

impl IntoStyle for Style {
    fn into_style(self) -> Style {
        self
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        *self
    }
}

impl IntoStyle for &'static str {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Value(Oco::Borrowed(self))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for String {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Value(Oco::Owned(self))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Rc<str> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Value(Oco::Counted(self))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (self).into_style()
    }
}

impl IntoStyle for Cow<'static, str> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Value(self.into())
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Oco<'static, str> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Value(self)
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Option<&'static str> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Option(self.map(Oco::Borrowed))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Option<String> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Option(self.map(Oco::Owned))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (self).into_style()
    }
}

impl IntoStyle for Option<Rc<str>> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Option(self.map(Oco::Counted))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Option<Cow<'static, str>> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Option(self.map(Oco::from))
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl IntoStyle for Option<Oco<'static, str>> {
    #[inline(always)]
    fn into_style(self) -> Style {
        Style::Option(self)
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl<T, U> IntoStyle for T
where
    T: Fn() -> U + 'static,
    U: IntoStyle,
{
    #[inline(always)]
    fn into_style(self) -> Style {
        let modified_fn = Rc::new(move || (self)().into_style());
        Style::Fn(modified_fn)
    }

    fn into_style_boxed(self: Box<Self>) -> Style {
        (*self).into_style()
    }
}

impl Style {
    /// Converts the style to its HTML value at that moment so it can be rendered on the server.
    pub fn as_value_string(
        &self,
        style_name: &str,
    ) -> Option<Oco<'static, str>> {
        match self {
            Style::Value(value) => {
                Some(format!("{style_name}: {value};").into())
            }
            Style::Option(value) => value
                .as_ref()
                .map(|value| format!("{style_name}: {value};").into()),
            Style::Fn(f) => {
                let mut value = f();
                while let Style::Fn(f) = value {
                    value = f();
                }
                value.as_value_string(style_name)
            }
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
#[inline(never)]
pub fn style_helper(
    el: &web_sys::Element,
    name: Oco<'static, str>,
    value: Style,
) {
    use leptos_reactive::create_render_effect;
    use std::ops::Deref;
    use wasm_bindgen::JsCast;

    let el = el.unchecked_ref::<web_sys::HtmlElement>();
    let style_list = el.style();
    match value {
        Style::Fn(f) => {
            create_render_effect(move |old| {
                let mut new = f();
                while let Style::Fn(f) = new {
                    new = f();
                }
                let new = match new {
                    Style::Value(value) => Some(value),
                    Style::Option(value) => value,
                    _ => unreachable!(),
                };
                if old.as_ref() != Some(&new) {
                    style_expression(&style_list, &name, new.as_deref(), true)
                }
                new
            });
        }
        Style::Value(value) => {
            style_expression(&style_list, &name, Some(value.deref()), false)
        }
        Style::Option(value) => {
            style_expression(&style_list, &name, value.as_deref(), false)
        }
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn style_expression(
    style_list: &web_sys::CssStyleDeclaration,
    style_name: &str,
    value: Option<&str>,
    force: bool,
) {
    use crate::HydrationCtx;

    if force || !HydrationCtx::is_hydrating() {
        let style_name = wasm_bindgen::intern(style_name);

        if let Some(value) = value {
            if let Err(e) = style_list.set_property(style_name, value) {
                crate::error!("[HtmlElement::style()] {e:?}");
            }
        } else {
            if let Err(e) = style_list.remove_property(style_name) {
                crate::error!("[HtmlElement::style()] {e:?}");
            }
        }
    }
}

macro_rules! style_type {
    ($style_type:ty) => {
        impl IntoStyle for $style_type {
            fn into_style(self) -> Style {
                Style::Value(self.to_string().into())
            }

            fn into_style_boxed(self: Box<Self>) -> Style {
                (*self).into_style()
            }
        }

        impl IntoStyle for Option<$style_type> {
            fn into_style(self) -> Style {
                Style::Option(self.map(|n| n.to_string().into()))
            }

            fn into_style_boxed(self: Box<Self>) -> Style {
                (*self).into_style()
            }
        }
    };
}

macro_rules! style_signal_type {
    ($signal_type:ty) => {
        #[cfg(not(feature = "nightly"))]
        impl<T> IntoStyle for $signal_type
        where
            T: IntoStyle + Clone,
        {
            fn into_style(self) -> Style {
                let modified_fn = Rc::new(move || self.get().into_style());
                Style::Fn(modified_fn)
            }

            fn into_style_boxed(self: Box<Self>) -> Style {
                (*self).into_style()
            }
        }
    };
}

macro_rules! style_signal_type_optional {
    ($signal_type:ty) => {
        #[cfg(not(feature = "nightly"))]
        impl<T> IntoStyle for $signal_type
        where
            T: Clone,
            Option<T>: IntoStyle,
        {
            fn into_style(self) -> Style {
                let modified_fn = Rc::new(move || self.get().into_style());
                Style::Fn(modified_fn)
            }

            fn into_style_boxed(self: Box<Self>) -> Style {
                (*self).into_style()
            }
        }
    };
}

style_type!(&String);
style_type!(usize);
style_type!(u8);
style_type!(u16);
style_type!(u32);
style_type!(u64);
style_type!(u128);
style_type!(isize);
style_type!(i8);
style_type!(i16);
style_type!(i32);
style_type!(i64);
style_type!(i128);
style_type!(f32);
style_type!(f64);
style_type!(char);

style_signal_type!(ReadSignal<T>);
style_signal_type!(RwSignal<T>);
style_signal_type!(Memo<T>);
style_signal_type!(Signal<T>);
style_signal_type!(MaybeSignal<T>);
style_signal_type_optional!(MaybeProp<T>);
