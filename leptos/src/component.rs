//! Utility traits and functions that allow building components,
//! as either functions of their props or functions with no arguments,
//! without knowing the name of the props struct.

pub trait Component<P> {}

pub trait Props {
    type Builder;

    fn builder() -> Self::Builder;
}

#[doc(hidden)]
pub trait PropsOrNoPropsBuilder {
    type Builder;

    fn builder_or_not() -> Self::Builder;
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyPropsBuilder {}

impl EmptyPropsBuilder {
    pub fn build(self) {}
}

impl<P: Props> PropsOrNoPropsBuilder for P {
    type Builder = <P as Props>::Builder;

    fn builder_or_not() -> Self::Builder {
        Self::builder()
    }
}

impl PropsOrNoPropsBuilder for EmptyPropsBuilder {
    type Builder = EmptyPropsBuilder;

    fn builder_or_not() -> Self::Builder {
        EmptyPropsBuilder {}
    }
}

impl<F, R> Component<EmptyPropsBuilder> for F where F: FnOnce() -> R {}

impl<P, F, R> Component<P> for F
where
    F: FnOnce(P) -> R,
    P: Props,
{
}

pub fn component_props_builder<P: PropsOrNoPropsBuilder>(
    _f: &impl Component<P>,
) -> <P as PropsOrNoPropsBuilder>::Builder {
    <P as PropsOrNoPropsBuilder>::builder_or_not()
}

pub fn component_view<P, T>(f: impl ComponentConstructor<P, T>, props: P) -> T {
    f.construct(props)
}
pub trait ComponentConstructor<P, T> {
    fn construct(self, props: P) -> T;
}

impl<Func, T> ComponentConstructor<(), T> for Func
where
    Func: FnOnce() -> T,
{
    fn construct(self, (): ()) -> T {
        (self)()
    }
}

impl<Func, T, P> ComponentConstructor<P, T> for Func
where
    Func: FnOnce(P) -> T,
    P: PropsOrNoPropsBuilder,
{
    fn construct(self, props: P) -> T {
        (self)(props)
    }
}
