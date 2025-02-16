use super::ChooseView;
use futures::FutureExt;
use std::{future::Future, pin::Pin};
use tachys::{erased::Erased, view::any_view::AnyView};

/// A type-erased [`ChooseView`].
pub struct AnyChooseView {
    value: Erased,
    clone: fn(&Erased) -> AnyChooseView,
    choose: fn(Erased) -> Pin<Box<dyn Future<Output = AnyView>>>,
    preload: for<'a> fn(&'a Erased) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
}

impl Clone for AnyChooseView {
    fn clone(&self) -> Self {
        (self.clone)(&self.value)
    }
}

impl AnyChooseView {
    pub(crate) fn new<T: ChooseView>(value: T) -> Self {
        fn clone<T: ChooseView>(value: &Erased) -> AnyChooseView {
            AnyChooseView::new(value.get_ref::<T>().clone())
        }

        fn choose<T: ChooseView>(
            value: Erased,
        ) -> Pin<Box<dyn Future<Output = AnyView>>> {
            value.into_inner::<T>().choose().boxed_local()
        }

        fn preload<'a, T: ChooseView>(
            value: &'a Erased,
        ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
            value.get_ref::<T>().preload().boxed_local()
        }

        Self {
            value: Erased::new(value),
            clone: clone::<T>,
            choose: choose::<T>,
            preload: preload::<T>,
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
