use crate::{runtime::with_runtime, update_view, PropValue};
use leptos_reactive::{
    create_effect, ReadSignal, RwSignal, Scope, SignalWith, WriteSignal,
};
use std::borrow::Cow;

pub trait PropValueFrom {
    fn into_prop_value(&self, cx: Scope) -> PropValue;
}

impl<T: ?Sized> PropValueFrom for T {
    default fn into_prop_value(&self, _cx: Scope) -> PropValue {
        PropValue::None
    }
}

impl<T> PropValueFrom for Vec<T> {
    default fn into_prop_value(&self, cx: Scope) -> PropValue {
        PropValue::Vec(self.iter().map(|v| v.into_prop_value(cx)).collect())
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

        PropValue::RwSignal(signal_hash)
    }
}

impl<T> PropValueFrom for WriteSignal<T> {
    fn into_prop_value(&self, _cx: Scope) -> PropValue {
        PropValue::WriteSignal
    }
}

impl<T> PropValueFrom for RwSignal<T> {
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

macro_rules! primitive_to_static {
    ($($child_type:ty),* $(,)?) => {
        $(
            impl PropValueFrom for $child_type {
                fn into_prop_value(&self, _cx: Scope) -> PropValue {
                    PropValue::Static(self.to_string())
                }
            }
        )*
    };
}

primitive_to_static![
    &str,
    String,
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    bool,
    Cow<'_, str>,
    std::net::IpAddr,
    std::net::SocketAddr,
    std::net::SocketAddrV4,
    std::net::SocketAddrV6,
    std::net::Ipv4Addr,
    std::net::Ipv6Addr,
    std::char::ToUppercase,
    std::char::ToLowercase,
    std::num::NonZeroI8,
    std::num::NonZeroU8,
    std::num::NonZeroI16,
    std::num::NonZeroU16,
    std::num::NonZeroI32,
    std::num::NonZeroU32,
    std::num::NonZeroI64,
    std::num::NonZeroU64,
    std::num::NonZeroI128,
    std::num::NonZeroU128,
    std::num::NonZeroIsize,
    std::num::NonZeroUsize,
    std::panic::Location<'_>,
    std::fmt::Arguments<'_>,
];

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
