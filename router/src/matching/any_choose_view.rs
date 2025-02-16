use super::ChooseView;
use futures::FutureExt;
use std::{any::Any, future::Future, pin::Pin};
use tachys::view::any_view::AnyView;

/// A type-erased [`ChooseView`].
pub struct AnyChooseView {
    value: Box<dyn Any + Send>,
    clone: fn(&Box<dyn Any + Send>) -> AnyChooseView,
    choose: fn(Box<dyn Any>) -> Pin<Box<dyn Future<Output = AnyView>>>,
    preload: for<'a> fn(
        &'a Box<dyn Any + Send>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
}

impl Clone for AnyChooseView {
    fn clone(&self) -> Self {
        (self.clone)(&self.value)
    }
}

impl AnyChooseView {
    pub(crate) fn new<T: ChooseView>(value: T) -> Self {
        Self {
            value: Box::new(value),
            clone: |value| {
                AnyChooseView::new(value.downcast_ref::<T>().unwrap().clone())
            },
            choose: |value| {
                value.downcast::<T>().unwrap().choose().boxed_local()
            },
            preload: |value| {
                value.downcast_ref::<T>().unwrap().preload().boxed_local()
            },
        }
    }
}

impl ChooseView for AnyChooseView {
    async fn choose(self) -> AnyView {
        (self.choose)(self.value).await
    }

    async fn preload(&self) {
        (self.preload)(&self.value).await;
    }
}
