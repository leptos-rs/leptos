#[doc(hidden)]
pub struct __IntoReactiveValueMarkerBaseCase;

/// A helper trait that works like `Into<T>` but uses a marker generic
/// to allow more `From` implementations than would be allowed with just `Into<T>`.
pub trait IntoReactiveValue<T, M> {
    /// Converts `self` into a `T`.
    fn into_reactive_value(self) -> T;
}

// The base case, which allows anything which implements .into() to work:
impl<T, I> IntoReactiveValue<T, __IntoReactiveValueMarkerBaseCase> for I
where
    I: Into<T>,
{
    fn into_reactive_value(self) -> T {
        self.into()
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        into_reactive_value::IntoReactiveValue,
        owner::{LocalStorage, Owner},
        traits::GetUntracked,
        wrappers::read::Signal,
    };
    use typed_builder::TypedBuilder;

    #[test]
    fn test_into_signal_compiles() {
        let owner = Owner::new();
        owner.set();

        #[cfg(not(feature = "nightly"))]
        let _: Signal<usize> = (|| 2).into_reactive_value();
        let _: Signal<usize, LocalStorage> = 2.into_reactive_value();
        #[cfg(not(feature = "nightly"))]
        let _: Signal<usize, LocalStorage> = (|| 2).into_reactive_value();
        let _: Signal<String> = "str".into_reactive_value();
        let _: Signal<String, LocalStorage> = "str".into_reactive_value();

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
        #[cfg(not(feature = "nightly"))]
        assert_eq!(Foo::builder().sig(|| 2).build().sig.get_untracked(), 2);
        assert_eq!(
            Foo::builder()
                .sig(Signal::stored(2))
                .build()
                .sig
                .get_untracked(),
            2
        );
    }
}
