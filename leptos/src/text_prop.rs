use leptos_reactive::Oco;
use std::{fmt::Debug, rc::Rc};

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(Rc<dyn Fn() -> Oco<'static, str>>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        (self.0)()
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
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<Rc<str>> for TextProp {
    fn from(s: Rc<str>) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp(Rc::new(move || s.clone()))
    }
}

impl<F, S> From<F> for TextProp
where
    F: Fn() -> S + 'static,
    S: Into<Oco<'static, str>>,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(Rc::new(move || s().into()))
    }
}
