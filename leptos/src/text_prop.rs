use leptos_reactive::{MaybeSignal, Oco, SignalGet};
use std::{fmt::Debug, rc::Rc};

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(MaybeSignal<Oco<'static, str>>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        self.0.get()
    }
}

impl Debug for TextProp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        let s: Oco<'_, str> = Oco::Counted(Rc::from(s));
        TextProp(MaybeSignal::derive(move || s.clone()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(MaybeSignal::derive(move || s.clone()))
    }
}

impl From<Rc<str>> for TextProp {
    fn from(s: Rc<str>) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(MaybeSignal::derive(move || s.clone()))
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp(MaybeSignal::derive(move || s.clone()))
    }
}

impl<F, I> From<F> for TextProp
where
    F: crate::Invocable<Value = I> + 'static,
    I: Into<Oco<'static, str>> + 'static + Clone,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(MaybeSignal::derive(move || s.invoke().into()))
    }
}

#[cfg(test)]
mod test {
    use super::{Oco, TextProp};
    use leptos_reactive::Signal;

    #[test]
    fn string_prop() {
        let s = String::new();
        let _prop: TextProp = s.into();
    }

    #[test]
    fn fn_oco_prop() {
        let s = || Oco::from("hi !");
        let _prop: TextProp = s.into();
    }

    #[test]
    fn signal_str_prop() {
        let s = Signal::derive(move || "hi !");
        let _prop: TextProp = s.into();
    }
}
