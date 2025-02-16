#![allow(clippy::type_complexity)]
use crate::{
    matching::any_choose_view::AnyChooseView, ChooseView, MatchInterface,
    MatchParams, RouteMatchId,
};
use std::{borrow::Cow, fmt::Debug};
use tachys::erased::ErasedLocal;

/// A type-erased container for any [`MatchParams'] + [`MatchInterface`].
pub struct AnyNestedMatch {
    value: ErasedLocal,
    to_params: fn(&ErasedLocal) -> Vec<(Cow<'static, str>, String)>,
    as_id: fn(&ErasedLocal) -> RouteMatchId,
    as_matched: for<'a> fn(&'a ErasedLocal) -> &'a str,
    into_view_and_child:
        fn(ErasedLocal) -> (AnyChooseView, Option<AnyNestedMatch>),
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
        let value = ErasedLocal::new(self);

        fn to_params<T: MatchParams + 'static>(
            value: &ErasedLocal,
        ) -> Vec<(Cow<'static, str>, String)> {
            let value = value.get_ref::<T>();
            value.to_params()
        }

        fn as_id<T: MatchInterface + 'static>(
            value: &ErasedLocal,
        ) -> RouteMatchId {
            let value = value.get_ref::<T>();
            value.as_id()
        }

        fn as_matched<T: MatchInterface + 'static>(
            value: &ErasedLocal,
        ) -> &str {
            let value = value.get_ref::<T>();
            value.as_matched()
        }

        fn into_view_and_child<T: MatchInterface + 'static>(
            value: ErasedLocal,
        ) -> (AnyChooseView, Option<AnyNestedMatch>) {
            let value = value.into_inner::<T>();
            let (view, child) = value.into_view_and_child();
            (
                AnyChooseView::new(view),
                child.map(|child| child.into_any_nested_match()),
            )
        }

        AnyNestedMatch {
            value,
            to_params: to_params::<T>,
            as_id: as_id::<T>,
            as_matched: as_matched::<T>,
            into_view_and_child: into_view_and_child::<T>,
        }
    }
}

impl MatchParams for AnyNestedMatch {
    fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
        (self.to_params)(&self.value)
    }
}

impl MatchInterface for AnyNestedMatch {
    type Child = AnyNestedMatch;

    fn as_id(&self) -> RouteMatchId {
        (self.as_id)(&self.value)
    }

    fn as_matched(&self) -> &str {
        (self.as_matched)(&self.value)
    }

    fn into_view_and_child(self) -> (impl ChooseView, Option<Self::Child>) {
        (self.into_view_and_child)(self.value)
    }
}
