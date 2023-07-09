use crate::{runtime::with_runtime, update_view, PropValue};
use leptos_reactive::{create_effect, ReadSignal, Scope, SignalWith};

pub trait PropValueFrom {
    fn into_prop_value(&self, cx: Scope) -> PropValue;
}

impl<T: ?Sized> PropValueFrom for T {
    default fn into_prop_value(&self, _cx: Scope) -> PropValue {
        PropValue::None
    }
}

impl PropValueFrom for bool {
    fn into_prop_value(&self, _cx: Scope) -> PropValue {
        PropValue::Static(self.to_string())
    }
}

impl<T> PropValueFrom for ReadSignal<T> {
    fn into_prop_value(&self, cx: Scope) -> PropValue {
        let signal = self.clone();

        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let mut hasher = DefaultHasher::default();
        signal.hash(&mut hasher);
        let signal_hash = hasher.finish();

        create_effect(cx, move |_| {
            let value = signal.try_with(move |v| v.into_prop_value(cx));
            if let Some(value) = value {
                update_signal(signal_hash, Some(value));
            } else {
                update_signal(signal_hash, None);
            }
        });

        PropValue::ReadSignal(signal_hash)
    }
}

pub(crate) fn update_signal(key: u64, value: Option<PropValue>) {
    with_runtime(|runtime| {
        if let Some(value) = value {
            runtime.signals.borrow_mut().insert(key, value);
        } else {
            runtime.signals.borrow_mut().remove(&key);
        }
    });
    update_view()
}
