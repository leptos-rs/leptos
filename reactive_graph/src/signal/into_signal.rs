use crate::{
    owner::{LocalStorage, Storage, SyncStorage},
    wrappers::read::Signal,
};

#[doc(hidden)]
pub struct __IntoSignalMarker1;
#[doc(hidden)]
pub struct __IntoSignalMarker2;
#[doc(hidden)]
pub struct __IntoSignalMarker3;

/// TODO docs
pub trait IntoSignal<T, M> {
    /// TODO docs
    fn into_signal(self) -> T;
}

impl<T, S, I: Into<Signal<T, S>>> IntoSignal<Signal<T, S>, __IntoSignalMarker1>
    for I
where
    S: Storage<T>,
{
    fn into_signal(self) -> Signal<T, S> {
        self.into()
    }
}

impl<T, F> IntoSignal<Signal<T, SyncStorage>, __IntoSignalMarker2> for F
where
    T: Send + Sync + 'static,
    F: Fn() -> T + Send + Sync + 'static,
{
    fn into_signal(self) -> Signal<T, SyncStorage> {
        Signal::derive(self)
    }
}

impl<T, F> IntoSignal<Signal<T, LocalStorage>, __IntoSignalMarker2> for F
where
    T: 'static,
    F: Fn() -> T + 'static,
{
    fn into_signal(self) -> Signal<T, LocalStorage> {
        Signal::derive_local(self)
    }
}

impl<F> IntoSignal<Signal<String, SyncStorage>, __IntoSignalMarker3> for F
where
    F: Fn() -> &'static str + Send + Sync + 'static,
{
    fn into_signal(self) -> Signal<String, SyncStorage> {
        Signal::derive(move || self().to_string())
    }
}

impl<F> IntoSignal<Signal<String, LocalStorage>, __IntoSignalMarker3> for F
where
    F: Fn() -> &'static str + 'static,
{
    fn into_signal(self) -> Signal<String, LocalStorage> {
        Signal::derive_local(move || self().to_string())
    }
}

#[cfg(test)]
mod tests {

    use typed_builder::TypedBuilder;

    use crate::{
        owner::LocalStorage, signal::into_signal::IntoSignal,
        traits::GetUntracked, wrappers::read::Signal,
    };

    #[test]
    fn text_into_signal_compiles() {
        fn my_to_signal<T, M>(val: impl IntoSignal<T, M>) -> T {
            val.into_signal()
        }

        fn f_usize(_sig: Signal<usize>) {}
        fn f_usize_local(_sig: Signal<usize, LocalStorage>) {}
        fn f_string(_sig: Signal<String>) {}
        fn f_string_local(_sig: Signal<String, LocalStorage>) {}

        f_usize(my_to_signal(2));
        f_usize(my_to_signal(|| 2));
        f_usize(my_to_signal(Signal::stored(2)));

        f_usize_local(my_to_signal(2));
        f_usize_local(my_to_signal(|| 2));
        f_usize_local(my_to_signal(Signal::stored_local(2)));

        f_string(my_to_signal("hello"));
        f_string(my_to_signal(|| "hello"));
        f_string(my_to_signal(Signal::stored("hello")));

        f_string_local(my_to_signal("hello"));
        f_string_local(my_to_signal(|| "hello"));
        f_string_local(my_to_signal(Signal::stored_local("hello")));

        #[derive(TypedBuilder)]
        struct Foo {
            #[builder(setter(transform_generics = "<M>", transform = |value: impl IntoSignal<Signal<usize>, M>| value.into_signal()))]
            sig: Signal<usize>,
        }

        assert_eq!(Foo::builder().sig(2).build().sig.get_untracked(), 2);
        assert_eq!(Foo::builder().sig(|| 2).build().sig.get_untracked(), 2);
    }
}
