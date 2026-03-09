//! Utility traits and functions that allow building components,
//! as either functions of their props or functions with no arguments,
//! without knowing the name of the props struct.

pub trait Component<P> {}

pub trait Props {
    type Builder;
    type Helper;

    fn builder() -> Self::Builder;
    fn helper() -> Self::Helper;
}

/// Unified access to a component's builder and helper structs, regardless of whether
/// the component takes props.
///
/// Two kinds of components exist:
/// - **With props**: `fn Foo(props: FooProps) -> impl IntoView`, matched by
///   `Component<FooProps>` where `FooProps: Props`. The blanket impl delegates
///   to `Props::builder()` / `Props::helper()`.
/// - **Without props**: `fn Foo() -> impl IntoView`, matched by
///   `Component<EmptyPropsBuilder>`. `EmptyPropsBuilder` doesn't implement
///   `Props` (there is no companion module to delegate to), so it gets its
///   own `ComponentAccess` impl returning no-op types (`NoHelper`,
///   `NoPresence`).
///
/// This trait unifies both paths so that `component_helper(&Foo)` works
/// uniformly for all components.
#[doc(hidden)]
pub trait ComponentAccess {
    type Builder;
    type Helper;

    fn access_builder() -> Self::Builder;
    fn access_helper() -> Self::Helper;
}

/// Placeholder props type for components that take no arguments.
#[doc(hidden)]
#[derive(Copy, Clone, Debug, Default)]
pub struct NoProps;

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Default)]
pub struct EmptyPropsBuilder {}

impl EmptyPropsBuilder {
    pub fn build(self) -> NoProps {
        NoProps
    }
}

impl<P: Props> ComponentAccess for P {
    type Builder = <P as Props>::Builder;
    type Helper = <P as Props>::Helper;

    fn access_builder() -> Self::Builder {
        Self::builder()
    }
    fn access_helper() -> Self::Helper {
        Self::helper()
    }
}

/// No-op helper for zero-prop components.
///
/// Zero-prop component functions take no arguments, so `Component<P>` resolves
/// with `P = EmptyPropsBuilder` (not a `Props` type). Since there is no
/// companion module, `NoHelper` provides the same API surface as a real
/// `Helper` (`.builder()`, `.presence()`) but returns trivial no-op types.
#[doc(hidden)]
pub struct NoHelper;

impl NoHelper {
    pub fn builder(&self) -> EmptyPropsBuilder {
        EmptyPropsBuilder {}
    }

    pub fn presence(&self) -> NoPresence {
        NoPresence
    }
}

/// No-op presence tracker for zero-prop components.
///
/// See [`NoHelper`] for context on why this exists.
#[doc(hidden)]
pub struct NoPresence;

impl NoPresence {
    pub fn require_props(&self) {}

    pub fn check_missing<T>(&self, builder: T) -> T {
        builder
    }
}

impl ComponentAccess for EmptyPropsBuilder {
    type Builder = EmptyPropsBuilder;
    type Helper = NoHelper;

    fn access_builder() -> Self::Builder {
        EmptyPropsBuilder {}
    }
    fn access_helper() -> Self::Helper {
        NoHelper
    }
}

impl<F, R> Component<EmptyPropsBuilder> for F where F: FnOnce() -> R {}

impl<P, F, R> Component<P> for F
where
    F: FnOnce(P) -> R,
    P: Props,
{
}

pub fn component_view<P, T>(f: impl ComponentConstructor<P, T>, props: P) -> T {
    f.construct(props)
}

/// Type-inference bridge which lets the compiler infer `P` from
/// `&impl Component<P>` so the `view!` macro can emit `component_helper(&Foo)`
/// without spelling out the props type (which may be `EmptyPropsBuilder`
/// for zero-prop components or carry generic parameters).
#[doc(hidden)]
pub fn component_helper<P: ComponentAccess>(
    _comp: &impl Component<P>,
) -> <P as ComponentAccess>::Helper {
    P::access_helper()
}

pub trait ComponentConstructor<P, T> {
    fn construct(self, props: P) -> T;
}

impl<Func, T> ComponentConstructor<NoProps, T> for Func
where
    Func: FnOnce() -> T,
{
    fn construct(self, _: NoProps) -> T {
        (self)()
    }
}

impl<Func, T, P> ComponentConstructor<P, T> for Func
where
    Func: FnOnce(P) -> T,
    P: ComponentAccess,
{
    fn construct(self, props: P) -> T {
        (self)(props)
    }
}
