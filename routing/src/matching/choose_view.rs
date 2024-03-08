use crate::RouteData;
use either_of::*;
use tachys::{renderer::Renderer, view::Render};

pub trait ChooseView<R>
where
    R: Renderer,
{
    type Output: Render<R>;

    fn choose(self, route_data: RouteData<R>) -> Self::Output;
}

impl<F, View, R> ChooseView<R> for F
where
    F: Fn(RouteData<R>) -> View,
    View: Render<R>,
    R: Renderer,
{
    type Output = View;

    fn choose(self, route_data: RouteData<R>) -> Self::Output {
        self(route_data)
    }
}

impl<R> ChooseView<R> for ()
where
    R: Renderer,
{
    type Output = ();

    fn choose(self, _route_data: RouteData<R>) -> Self::Output {}
}

impl<A, B, Rndr> ChooseView<Rndr> for Either<A, B>
where
    A: ChooseView<Rndr>,
    B: ChooseView<Rndr>,
    Rndr: Renderer,
{
    type Output = Either<A::Output, B::Output>;

    fn choose(self, route_data: RouteData<Rndr>) -> Self::Output {
        match self {
            Either::Left(f) => Either::Left(f.choose(route_data)),
            Either::Right(f) => Either::Right(f.choose(route_data)),
        }
    }
}

// TODO add other Either implementations
