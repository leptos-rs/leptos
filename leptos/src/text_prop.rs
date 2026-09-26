use oco_ref::Oco;
use std::sync::Arc;
use tachys::prelude::IntoAttributeValue;

/// Describes a value that is either a static or a reactive string, i.e.,
/// a [`String`], a [`&str`], a `Signal` or a reactive `Fn() -> String`.
#[derive(Clone)]
pub enum TextProp {
    /// A static (or owned) string. Reading it is an [`Oco`] clone (a pointer
    /// bump for `Borrowed`/`Counted`), with no allocation or dynamic dispatch.
    Static(Oco<'static, str>),
    /// A reactive value, behind a reference-counted closure.
    Fn(Arc<dyn Fn() -> Oco<'static, str> + Send + Sync>),
}

impl TextProp {
    /// Accesses the current value of the property.
    #[inline(always)]
    pub fn get(&self) -> Oco<'static, str> {
        match self {
            TextProp::Static(value) => value.clone(),
            TextProp::Fn(f) => f(),
        }
    }
}

impl core::fmt::Debug for TextProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        TextProp::Static(Oco::Counted(Arc::from(s)))
    }
}

impl From<&'static str> for TextProp {
    fn from(s: &'static str) -> Self {
        TextProp::Static(s.into())
    }
}

impl From<Arc<str>> for TextProp {
    fn from(s: Arc<str>) -> Self {
        TextProp::Static(s.into())
    }
}

impl From<Oco<'static, str>> for TextProp {
    fn from(s: Oco<'static, str>) -> Self {
        TextProp::Static(s)
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
        TextProp::Fn(Arc::new(move || s().into()))
    }
}

impl Default for TextProp {
    fn default() -> Self {
        TextProp::Static(Oco::Borrowed(""))
    }
}

impl IntoAttributeValue for TextProp {
    type Output = Arc<dyn Fn() -> Oco<'static, str> + Send + Sync>;

    fn into_attribute_value(self) -> Self::Output {
        match self {
            TextProp::Static(value) => Arc::new(move || value.clone()),
            TextProp::Fn(f) => f,
        }
    }
}

#[allow(unused)]
macro_rules! textprop_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> From<$name<$($gen),*>> for TextProp
        where
            $v: Into<Oco<'static, str>>  + Clone + Send + Sync + 'static,
            $($where_clause)*
        {
            #[inline(always)]
            fn from(s: $name<$($gen),*>) -> Self {
                TextProp::Fn(Arc::new(move || s.get().into()))
            }
        }
    };
}

mod stable {
    use super::TextProp;
    use oco_ref::Oco;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use std::sync::Arc;

    textprop_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    textprop_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    textprop_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    textprop_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    textprop_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

/// Extension trait for `Option<TextProp>`
pub trait OptionTextPropExt {
    /// Accesses the current value of the `Option<TextProp>` as an `Option<Oco<'static, str>>`.
    fn get(&self) -> Option<Oco<'static, str>>;
}

impl OptionTextPropExt for Option<TextProp> {
    fn get(&self) -> Option<Oco<'static, str>> {
        self.as_ref().map(|text_prop| text_prop.get())
    }
}

#[cfg(test)]
mod tests {
    use super::TextProp;
    use tachys::prelude::IntoAttributeValue;

    #[test]
    fn variants_read_back_their_value() {
        assert_eq!(&*TextProp::from("literal").get(), "literal");
        assert_eq!(&*TextProp::from(String::from("owned")).get(), "owned");
        assert_eq!(&*TextProp::from(|| "dynamic").get(), "dynamic");
        assert_eq!(&*TextProp::default().get(), "");
    }

    #[test]
    fn static_inputs_use_the_non_boxed_variant() {
        assert!(matches!(TextProp::from("literal"), TextProp::Static(_)));
        assert!(matches!(
            TextProp::from(String::from("owned")),
            TextProp::Static(_)
        ));
        assert!(matches!(TextProp::from(|| "dynamic"), TextProp::Fn(_)));
    }

    #[test]
    fn into_attribute_value_preserves_the_value() {
        let f = TextProp::from("literal").into_attribute_value();
        assert_eq!(&*f(), "literal");
        let f = TextProp::from(|| "dynamic").into_attribute_value();
        assert_eq!(&*f(), "dynamic");
    }
}
