use oco_ref::Oco;
use std::sync::Arc;
use tachys::prelude::IntoAttributeValue;

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], or a reactive `Fn() -> String`.
#[derive(Clone)]
pub struct TextProp(Arc<dyn Fn() -> Oco<'static, str> + Send + Sync>);

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        (self.0)()
    }
}

impl core::fmt::Debug for TextProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        let s: Oco<'_, str> = Oco::Counted(Arc::from(s));
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<Arc<str>> for TextProp {
    fn from(s: Arc<str>) -> Self {
        let s: Oco<'_, str> = s.into();
        TextProp(Arc::new(move || s.clone()))
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp(Arc::new(move || s.clone()))
    }
}

// TODO
/*impl<T> From<T> for MaybeProp<TextProp>
where
    T: Into<Oco<'static, str>>,
{
    fn from(s: T) -> Self {
        Self(Some(MaybeSignal::from(Some(s.into().into()))))
    }
}*/

impl<F, S> From<F> for TextProp
where
    F: Fn() -> S + 'static + Send + Sync,
    S: Into<Oco<'static, str>>,
{
    #[inline(always)]
    fn from(s: F) -> Self {
        TextProp(Arc::new(move || s().into()))
    }
}

impl Default for TextProp {
    fn default() -> Self {
        Self(Arc::new(|| Oco::Borrowed("")))
    }
}

impl IntoAttributeValue for TextProp {
    type Output = Oco<'static, str>;

    fn into_attribute_value(self) -> Self::Output {
        self.get()
    }
}
