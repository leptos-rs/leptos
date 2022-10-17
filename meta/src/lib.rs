use std::fmt::Debug;

use leptos::*;

mod stylesheet;
mod title;
pub use stylesheet::*;
pub use title::*;

#[derive(Debug, Clone, Default)]
pub struct MetaContext {
    pub(crate) title: TitleContext,
    pub(crate) stylesheets: StylesheetContext,
}

pub fn use_head(cx: Scope) -> MetaContext {
    match use_context::<MetaContext>(cx) {
        None => {
            log::warn!("use_head() can only be called if a MetaContext has been provided");
            panic!()
        }
        Some(ctx) => ctx,
    }
}

impl MetaContext {
    pub fn new() -> Self {
        Default::default()
    }

    #[cfg(feature = "ssr")]
    pub fn dehydrate(&self) -> String {
        let mut tags = String::new();

        // Title
        if let Some(title) = self.title.as_string() {
            tags.push_str("<title>");
            tags.push_str(&title);
            tags.push_str("</title>");
        }

        // Stylesheets
        tags.push_str(&self.stylesheets.as_string());

        tags
    }
}

pub struct TextProp(Box<dyn Fn() -> String>);

impl Debug for TextProp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        TextProp(Box::new(move || s.clone()))
    }
}

impl From<&str> for TextProp {
    fn from(s: &str) -> Self {
        let s = s.to_string();
        TextProp(Box::new(move || s.clone()))
    }
}

impl<F> From<F> for TextProp
where
    F: Fn() -> String + 'static,
{
    fn from(s: F) -> Self {
        TextProp(Box::new(s))
    }
}
