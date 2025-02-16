use crate::{
    matching::any_choose_view::AnyChooseView, ChooseView, MatchInterface,
    MatchParams, RouteMatchId,
};
use std::{any::Any, borrow::Cow, fmt::Debug};

/// A type-erased container for any [`MatchParams'] + [`MatchInterface`].
pub struct AnyNestedMatch {
    value: Box<dyn Any>,
    to_params: fn(&dyn Any) -> Vec<(Cow<'static, str>, String)>,
    as_id: fn(&dyn Any) -> RouteMatchId,
    as_matched: for<'a> fn(&'a dyn Any) -> &'a str,
    into_view_and_child:
        fn(Box<dyn Any>) -> (AnyChooseView, Option<AnyNestedMatch>),
}

impl Debug for AnyNestedMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyNestedMatch").finish_non_exhaustive()
    }
}

/// Converts anything implementing [`MatchParams'] + [`MatchInterface`] into an erased type.
pub trait IntoAnyNestedMatch {
    /// Wraps the nested route.
    fn into_any_nested_match(self) -> AnyNestedMatch;
}

impl<T> IntoAnyNestedMatch for T
where
    T: MatchParams + MatchInterface + 'static,
{
    fn into_any_nested_match(self) -> AnyNestedMatch {
        let value = Box::new(self) as Box<dyn Any>;
        let value = match (value as Box<dyn Any>).downcast::<AnyNestedMatch>() {
            // if it's already an AnyNestedMatch, we don't need to double-wrap it
            Ok(any_nested_route) => return *any_nested_route,
            Err(value) => value.downcast::<T>().unwrap(),
        };

        let to_params = |value: &dyn Any| {
            let value = value
                .downcast_ref::<T>()
                .expect("AnyNestedMatch::to_params couldn't downcast");
            value.to_params()
        };

        let as_id = |value: &dyn Any| {
            let value = value
                .downcast_ref::<T>()
                .expect("AnyNestedMatch::as_id couldn't downcast");
            value.as_id()
        };

        fn as_matched<'a, T: MatchInterface + 'static>(
            value: &'a dyn Any,
        ) -> &'a str {
            let value = value
                .downcast_ref::<T>()
                .expect("AnyNestedMatch::as_matched couldn't downcast");
            value.as_matched()
        }

        let into_view_and_child = |value: Box<dyn Any>| {
            let value = value.downcast::<T>().expect(
                "AnyNestedMatch::into_view_and_child couldn't downcast",
            );
            let (view, child) = value.into_view_and_child();
            (
                AnyChooseView::new(view),
                child.map(|child| child.into_any_nested_match()),
            )
        };

        AnyNestedMatch {
            value,
            to_params,
            as_id,
            as_matched: as_matched::<T>,
            into_view_and_child,
        }
    }
}

impl MatchParams for AnyNestedMatch {
    fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
        (self.to_params)(&*self.value)
    }
}

impl MatchInterface for AnyNestedMatch {
    type Child = AnyNestedMatch;

    fn as_id(&self) -> RouteMatchId {
        (self.as_id)(&*self.value)
    }

    fn as_matched(&self) -> &str {
        (self.as_matched)(&*self.value)
    }

    fn into_view_and_child(self) -> (impl ChooseView, Option<Self::Child>) {
        (self.into_view_and_child)(self.value)
    }
}
