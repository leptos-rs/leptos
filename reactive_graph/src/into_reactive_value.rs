use crate::{
    computed::{ArcMemo, Memo},
    owner::Storage,
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    wrappers::read::{ArcSignal, Signal},
};

#[doc(hidden)]
pub struct __IntoReactiveValueMarkerBaseCase;
#[doc(hidden)]
pub struct __IntoReactiveValueMarkerIdentity;

/// A helper trait that works like `Into<T>` but uses a marker generic
/// to allow more `From` implementations than would be allowed with just `Into<T>`.
pub trait IntoReactiveValue<T, M> {
    /// Converts `self` into a `T`.
    fn into_reactive_value(self) -> T;
}

// The base case, which allows anything which implements .into() to work.
impl<T, I> IntoReactiveValue<T, __IntoReactiveValueMarkerBaseCase> for I
where
    I: Into<T>,
{
    fn into_reactive_value(self) -> T {
        self.into()
    }
}

/// A systemic fix for ambiguity: a trait that picks the best conversion automatically.
pub trait IntoReactiveValueTrait<T, M> {
    /// Converts `self` into a `T`.
    fn into_reactive_value(self) -> T;
}

impl<T, I> IntoReactiveValueTrait<T, __IntoReactiveValueMarkerBaseCase> for I
where
    I: Into<T>,
{
    fn into_reactive_value(self) -> T {
        self.into()
    }
}

impl<T, S, I>
    IntoReactiveValueTrait<Signal<T, S>, __IntoReactiveValueMarkerIdentity>
    for I
where
    I: IntoReactiveValue<Signal<T, S>, __IntoReactiveValueMarkerIdentity>,
    S: Storage<T>,
{
    fn into_reactive_value(self) -> Signal<T, S> {
        self.into_reactive_value()
    }
}

impl<T, S, I>
    IntoReactiveValueTrait<ArcSignal<T, S>, __IntoReactiveValueMarkerIdentity>
    for I
where
    I: IntoReactiveValue<ArcSignal<T, S>, __IntoReactiveValueMarkerIdentity>,
    S: Storage<T>,
{
    fn into_reactive_value(self) -> ArcSignal<T, S> {
        self.into_reactive_value()
    }
}

impl<T, F>
    IntoReactiveValueTrait<
        Signal<T>,
        crate::wrappers::read::__IntoReactiveValueMarkerSignalFromReactiveClosure,
    > for F
where
    T: Send + Sync + 'static,
    F: IntoReactiveValue<
        Signal<T>,
        crate::wrappers::read::__IntoReactiveValueMarkerSignalFromReactiveClosure,
    >,
{
    fn into_reactive_value(self) -> Signal<T> {
        self.into_reactive_value()
    }
}

/// Specialized trait for signals to resolve ambiguity.
pub trait IntoSignal<T, M> {
    /// Converts `self` into a `T`.
    fn into_signal(self) -> T;
}

// Implement for Signal itself (Identity)
impl<T, S> IntoSignal<Signal<T, S>, __IntoReactiveValueMarkerIdentity>
    for Signal<T, S>
where
    S: Storage<T>,
{
    fn into_signal(self) -> Signal<T, S> {
        self
    }
}

// Generic implementation via IntoReactiveValue (Identity path)
impl<T, S, I> IntoSignal<Signal<T, S>, __IntoReactiveValueMarkerIdentity> for I
where
    I: IntoReactiveValue<Signal<T, S>, __IntoReactiveValueMarkerIdentity>,
    S: Storage<T>,
{
    fn into_signal(self) -> Signal<T, S> {
        self.into_reactive_value()
    }
}

// ArcSignal Identity
impl<T, S> IntoSignal<ArcSignal<T, S>, __IntoReactiveValueMarkerIdentity>
    for ArcSignal<T, S>
where
    S: Storage<T>,
{
    fn into_signal(self) -> ArcSignal<T, S> {
        self
    }
}

impl<T, S, I> IntoSignal<ArcSignal<T, S>, __IntoReactiveValueMarkerIdentity>
    for I
where
    I: IntoReactiveValue<ArcSignal<T, S>, __IntoReactiveValueMarkerIdentity>,
    S: Storage<T>,
{
    fn into_signal(self) -> ArcSignal<T, S> {
        self.into_reactive_value()
    }
}

// Implement for specific signal types to allow identity conversion
impl<T, S> IntoSignal<RwSignal<T, S>, __IntoReactiveValueMarkerIdentity>
    for RwSignal<T, S>
where
    S: Storage<ArcRwSignal<T>> + Storage<T>,
{
    fn into_signal(self) -> RwSignal<T, S> {
        self
    }
}

impl<T, S> IntoSignal<ReadSignal<T, S>, __IntoReactiveValueMarkerIdentity>
    for ReadSignal<T, S>
where
    S: Storage<ArcReadSignal<T>> + Storage<T>,
{
    fn into_signal(self) -> ReadSignal<T, S> {
        self
    }
}

impl<T, S> IntoSignal<Memo<T, S>, __IntoReactiveValueMarkerIdentity>
    for Memo<T, S>
where
    S: Storage<ArcMemo<T, S>> + Storage<T>,
{
    fn into_signal(self) -> Memo<T, S> {
        self
    }
}

// Closure path
impl<T, F>
    IntoSignal<
        Signal<T>,
        crate::wrappers::read::__IntoReactiveValueMarkerSignalFromReactiveClosure,
    > for F
where
    T: Send + Sync + 'static,
    F: IntoReactiveValue<
        Signal<T>,
        crate::wrappers::read::__IntoReactiveValueMarkerSignalFromReactiveClosure,
    >,
{
    fn into_signal(self) -> Signal<T> {
        self.into_reactive_value()
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        computed::{ArcMemo, Memo},
        into_reactive_value::IntoReactiveValue,
        owner::{LocalStorage, Owner},
        signal::{ArcRwSignal, RwSignal},
        traits::{Get, GetUntracked},
        wrappers::read::{ArcSignal, Signal},
    };
    use typed_builder::TypedBuilder;

    #[test]
    fn test_into_signal_compiles() {
        let owner = Owner::new();
        owner.set();

        let _: Signal<usize> = (|| 2).into_reactive_value();
        let _: Signal<usize, LocalStorage> = 2.into_reactive_value();
        let _: Signal<usize, LocalStorage> = (|| 2).into_reactive_value();
        let _: Signal<String> = "str".into_reactive_value();
        let _: Signal<String, LocalStorage> = "str".into_reactive_value();

        // Closures capturing signal types work because
        // the Fn-based impls have no restricting bound.
        {
            let a: Signal<usize> = (|| 2).into_reactive_value();
            let b: Signal<usize> = Signal::stored(2).into_reactive_value();
            let _: Signal<usize> =
                (move || a.get() + b.get()).into_reactive_value();
        }

        #[derive(TypedBuilder)]
        struct Foo {
            #[builder(setter(
                fn transform<M>(value: impl IntoReactiveValue<Signal<usize>, M>) {
                    value.into_reactive_value()
                }
            ))]
            sig: Signal<usize>,
        }

        assert_eq!(Foo::builder().sig(2).build().sig.get_untracked(), 2);
        assert_eq!(Foo::builder().sig(|| 2).build().sig.get_untracked(), 2);
        assert_eq!(
            Foo::builder()
                .sig(Signal::stored(2))
                .build()
                .sig
                .get_untracked(),
            2
        );

        // Signal types go through Fn-based impls (Signal::derive).
        {
            let rw = RwSignal::new(42usize);
            let sig: Signal<usize> = Foo::builder().sig(rw).build().sig;
            assert_eq!(sig.get_untracked(), 42);
        }
    }

    /// Regression test: every signal type that has a From<X> for Signal<T>
    /// impl must be convertible via into_reactive_value().
    /// These go through the Into-based base case.
    #[test]
    fn signal_types_into_reactive_value() {
        let owner = Owner::new();
        owner.set();

        // ReadSignal -> Signal
        let (r, _) = crate::signal::signal(42usize);
        let sig: Signal<usize> = r.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // ArcReadSignal -> Signal
        let (ar, _) = crate::signal::arc_signal(42usize);
        let sig: Signal<usize> = ar.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // RwSignal -> Signal
        let rw = RwSignal::new(42usize);
        let sig: Signal<usize> = rw.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // ArcRwSignal -> Signal
        let arw = ArcRwSignal::new(42usize);
        let sig: Signal<usize> = arw.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // Memo -> Signal
        let memo = Memo::new(|_| 42usize);
        let sig: Signal<usize> = memo.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // ArcMemo -> Signal
        let arc_memo = ArcMemo::new(|_| 42usize);
        let sig: Signal<usize> = arc_memo.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // Signal -> Signal (identity via Into)
        let s = Signal::stored(42usize);
        let sig: Signal<usize> = s.into_reactive_value();
        assert_eq!(sig.get_untracked(), 42);

        // Derived signals can capture other signals and still be converted
        let derived = Signal::derive(move || s.get() + s.get());
        let sig: Signal<usize> = derived.into_reactive_value();
        assert_eq!(sig.get_untracked(), 84);

        // (Closure) derived signals can capture Signal and still be converted
        let derived = move || s.get() + s.get();
        let sig: Signal<usize> = derived.into_reactive_value();
        assert_eq!(sig.get_untracked(), 84);

        // (Closure) derived signals can capture other signals and still be converted
        let rw = RwSignal::new(42usize);
        let derived = move || rw.get() + s.get();
        let sig: Signal<usize> = derived.into_reactive_value();
        assert_eq!(sig.get_untracked(), 84);

        // see https://github.com/leptos-rs/leptos/pull/4617#issuecomment-4014829699
        let a: ArcSignal<usize> = (|| 2).into_reactive_value();
        let b: ArcSignal<usize> = ArcSignal::stored(2).into_reactive_value();
        let _: ArcSignal<usize> =
            (move || a.get() + b.get()).into_reactive_value();
    }
}
