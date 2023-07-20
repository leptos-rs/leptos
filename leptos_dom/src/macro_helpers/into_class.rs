/// Represents the different possible values a single class on an element could have,
/// allowing you to do fine-grained updates to single items
/// in [`Element.classList`](https://developer.mozilla.org/en-US/docs/Web/API/Element/classList).
///
/// This mostly exists for the [`view`](https://docs.rs/leptos_macro/latest/leptos_macro/macro.view.html)
/// macro’s use. You usually won't need to interact with it directly, but it can be useful for defining
/// permissive APIs for certain components.
pub enum Class {
    /// Whether the class is present.
    Value(bool),
    /// A (presumably reactive) function, which will be run inside an effect to toggle the class.
    Fn(Box<dyn Fn() -> bool>),
}

/// Converts some type into a [Class].
pub trait IntoClass {
    /// Converts the object into a [Class].
    fn into_class(self) -> Class;
}

impl IntoClass for bool {
    #[inline(always)]
    fn into_class(self) -> Class {
        Class::Value(self)
    }
}

impl<T> IntoClass for T
where
    T: Fn() -> bool + 'static,
{
    #[inline(always)]
    fn into_class(self) -> Class {
        let modified_fn = Box::new(self);
        Class::Fn(modified_fn)
    }
}

impl Class {
    /// Converts the class to its HTML value at that moment so it can be rendered on the server.
    pub fn as_value_string(&self, class_name: &'static str) -> &'static str {
        match self {
            Class::Value(value) => {
                if *value {
                    class_name
                } else {
                    ""
                }
            }
            Class::Fn(f) => {
                let value = f();
                if value {
                    class_name
                } else {
                    ""
                }
            }
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::borrow::Cow;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
#[inline(never)]
pub fn class_helper(
    el: &web_sys::Element,
    name: Cow<'static, str>,
    value: Class,
) {
    use leptos_reactive::create_render_effect;

    let class_list = el.class_list();
    match value {
        Class::Fn(f) => {
            create_render_effect(move |old| {
                let new = f();
                if old.as_ref() != Some(&new) && (old.is_some() || new) {
                    class_expression(&class_list, &name, new, true)
                }
                new
            });
        }
        Class::Value(value) => {
            class_expression(&class_list, &name, value, false)
        }
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[inline(never)]
pub(crate) fn class_expression(
    class_list: &web_sys::DomTokenList,
    class_name: &str,
    value: bool,
    force: bool,
) {
    use crate::HydrationCtx;

    if force || !HydrationCtx::is_hydrating() {
        let class_name = wasm_bindgen::intern(class_name);

        if value {
            if let Err(e) = class_list.add_1(class_name) {
                crate::error!("[HtmlElement::class()] {e:?}");
            }
        } else {
            if let Err(e) = class_list.remove_1(class_name) {
                crate::error!("[HtmlElement::class()] {e:?}");
            }
        }
    }
}
