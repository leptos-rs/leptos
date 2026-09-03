/// Implemented for all concrete signal types and for “derived signals” (closures
/// that read from signals).
///
/// This is intended for use in component prop bounds. It allows a component to
/// accept a prop that can be either a signal, or a closure that reads from signals,
/// on both stable and nightly.
pub trait SignalOrFn {
    /// The value produced (reactively) when invoked.
    type Output;

    /// Reactively read the current value (subscribes the active observer).
    fn run(&self) -> Self::Output;
}

impl<F, T> SignalOrFn for F
where
    F: Fn() -> T,
{
    type Output = T;

    fn run(&self) -> T {
        self()
    }
}

#[cfg(test)]
mod tests {
    use super::SignalOrFn;
    #[allow(deprecated)]
    use crate::wrappers::read::MaybeSignal;
    use crate::{
        computed::Memo,
        owner::Owner,
        signal::{ArcRwSignal, RwSignal},
        traits::Get,
        wrappers::read::{MaybeProp, Signal},
    };

    fn collect<S>(each: S) -> Vec<i32>
    where
        S: SignalOrFn<Output = Vec<i32>>,
    {
        each.run()
    }

    #[test]
    fn accepts_signal_or_closure() {
        let owner = Owner::new();
        owner.set();

        // a closure
        assert_eq!(collect(|| vec![1, 2, 3]), vec![1, 2, 3]);

        // a concrete signal
        let sig = RwSignal::new(vec![4, 5, 6]);
        assert_eq!(collect(sig), vec![4, 5, 6]);

        // a Signal<_> wrapper
        let wrapped: Signal<Vec<i32>> = Signal::derive(move || vec![7, 8]);
        assert_eq!(collect(wrapped), vec![7, 8]);

        // closure capturing a signal still works (it's just a Fn)
        let src = RwSignal::new(vec![9]);
        assert_eq!(collect(move || src.get()), vec![9]);
    }

    #[test]
    #[allow(deprecated)]
    fn coverage_across_signal_types() {
        let owner = Owner::new();
        owner.set();

        fn accepts_signal_or_fn<S: SignalOrFn<Output = i32>>(s: S) -> i32 {
            s.run()
        }
        fn accepts_signal_or_fn_opt<S: SignalOrFn<Output = Option<i32>>>(
            s: S,
        ) -> Option<i32> {
            s.run()
        }

        assert_eq!(accepts_signal_or_fn(RwSignal::new(1)), 1);
        assert_eq!(accepts_signal_or_fn(Memo::new(|_| 2)), 2);
        assert_eq!(accepts_signal_or_fn(Signal::derive(|| 3)), 3);
        assert_eq!(accepts_signal_or_fn(MaybeSignal::from(4)), 4);
        assert_eq!(accepts_signal_or_fn(ArcRwSignal::new(5)), 5);
        assert_eq!(accepts_signal_or_fn_opt(MaybeProp::from(6)), Some(6));
        assert_eq!(accepts_signal_or_fn(|| 7), 7);
    }
}
