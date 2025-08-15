use either_of::*;
use leptos::prelude::{ArcStoredValue, WriteValue};
use std::{future::Future, marker::PhantomData};
use tachys::view::any_view::{AnyView, IntoAny};

pub trait ChooseView
where
    Self: Send + Clone + 'static,
{
    fn choose(self) -> impl Future<Output = AnyView>;

    fn preload(&self) -> impl Future<Output = ()>;
}

impl<F, View> ChooseView for F
where
    F: Fn() -> View + Send + Clone + 'static,
    View: IntoAny,
{
    async fn choose(self) -> AnyView {
        self().into_any()
    }

    async fn preload(&self) {}
}

impl<T> ChooseView for Lazy<T>
where
    T: Send + Sync + LazyRoute,
{
    async fn choose(self) -> AnyView {
        let data = self.data.write_value().take().unwrap_or_else(T::data);
        T::view(data).await
    }

    async fn preload(&self) {
        *self.data.write_value() = Some(T::data());
        T::preload().await;
    }
}

pub trait LazyRoute: Send + 'static {
    fn data() -> Self;

    fn view(this: Self) -> impl Future<Output = AnyView>;

    fn preload() -> impl Future<Output = ()> {
        async {}
    }
}

#[derive(Debug)]
pub struct Lazy<T> {
    ty: PhantomData<T>,
    data: ArcStoredValue<Option<T>>,
}

impl<T> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        Self {
            ty: self.ty,
            data: self.data.clone(),
        }
    }
}

impl<T> Lazy<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Self {
            ty: Default::default(),
            data: ArcStoredValue::new(None),
        }
    }
}

impl ChooseView for () {
    async fn choose(self) -> AnyView {
        ().into_any()
    }

    async fn preload(&self) {}
}

impl<A, B> ChooseView for Either<A, B>
where
    A: ChooseView,
    B: ChooseView,
{
    async fn choose(self) -> AnyView {
        match self {
            Either::Left(f) => f.choose().await.into_any(),
            Either::Right(f) => f.choose().await.into_any(),
        }
    }

    async fn preload(&self) {
        match self {
            Either::Left(f) => f.preload().await,
            Either::Right(f) => f.preload().await,
        }
    }
}

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        impl<$($ty,)*> ChooseView for $either<$($ty,)*>
        where
            $($ty: ChooseView,)*
        {
            async fn choose(self) -> AnyView {
                match self {
                    $(
                        $either::$ty(f) => f.choose().await.into_any(),
                    )*
                }
            }

            async fn preload(&self) {
                match self {
                    $($either::$ty(f) => f.preload().await,)*
                }
            }
        }
    };
}

tuples!(EitherOf3 => A, B, C);
tuples!(EitherOf4 => A, B, C, D);
tuples!(EitherOf5 => A, B, C, D, E);
tuples!(EitherOf6 => A, B, C, D, E, F);
tuples!(EitherOf7 => A, B, C, D, E, F, G);
tuples!(EitherOf8 => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// A version of [`IntoMaybeErased`] for the [`ChooseView`] trait.
pub trait IntoChooseViewMaybeErased {
    /// The type of the erased view.
    type Output: IntoChooseViewMaybeErased;

    /// Erase the type of the view.
    fn into_maybe_erased(self) -> Self::Output;
}

impl<T> IntoChooseViewMaybeErased for T
where
    T: ChooseView + Send + Clone + 'static,
{
    #[cfg(erase_components)]
    type Output = crate::matching::any_choose_view::AnyChooseView;

    #[cfg(not(erase_components))]
    type Output = Self;

    fn into_maybe_erased(self) -> Self::Output {
        #[cfg(erase_components)]
        {
            crate::matching::any_choose_view::AnyChooseView::new(self)
        }
        #[cfg(not(erase_components))]
        {
            self
        }
    }
}
