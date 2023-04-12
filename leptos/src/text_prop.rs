use std::{fmt::Debug, rc::Rc};

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [String], a [&str], or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(Rc<dyn Fn() -> String>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> String {
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
        TextProp(Rc::new(move || s.clone()))
    }
}

impl From<&str> for TextProp {
    fn from(s: &str) -> Self {
        let s = s.to_string();
        TextProp(Rc::new(move || s.clone()))
    }
}

impl<F> From<F> for TextProp
where
    F: Fn() -> String + 'static,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(Rc::new(s))
    }
}
