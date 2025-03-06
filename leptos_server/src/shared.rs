use crate::{FromEncodedStr, IntoEncodedString};
#[cfg(feature = "rkyv")]
use codee::binary::RkyvCodec;
#[cfg(feature = "serde-wasm-bindgen")]
use codee::string::JsonSerdeWasmCodec;
#[cfg(feature = "miniserde")]
use codee::string::MiniserdeCodec;
#[cfg(feature = "serde-lite")]
use codee::SerdeLite;
use codee::{
    string::{FromToStringCodec, JsonSerdeCodec},
    Decoder, Encoder,
};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A smart pointer that allows you to share identical, synchronously-loaded data between the
/// server and the client.
///
/// If this constructed on the server, it serializes its value into the shared context. If it is
/// constructed on the client during hydration, it reads its value from the shared context. If
/// it it constructed on the client at any other time, it simply runs on the client.
#[derive(Debug)]
pub struct SharedValue<T, Ser = JsonSerdeCodec> {
    value: T,
    ser: PhantomData<Ser>,
}

impl<T, Ser> SharedValue<T, Ser> {
    /// Returns the inner value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> SharedValue<T, JsonSerdeCodec>
where
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeCodec as Decoder<T>>::Error: Debug,
    <JsonSerdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
    <<JsonSerdeCodec as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`JsonSerdeCodec`] encoding.
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

impl<T> SharedValue<T, FromToStringCodec>
where
    FromToStringCodec: Encoder<T> + Decoder<T>,
    <FromToStringCodec as Encoder<T>>::Error: Debug,
    <FromToStringCodec as Decoder<T>>::Error: Debug,
    <FromToStringCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <FromToStringCodec as Decoder<T>>::Encoded: FromEncodedStr,
    <<FromToStringCodec as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`FromToStringCodec`] encoding.
    pub fn new_str(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> SharedValue<T, SerdeLite<JsonSerdeCodec>>
where
    SerdeLite<JsonSerdeCodec>: Encoder<T> + Decoder<T>,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Error: Debug,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Error: Debug,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Encoded: IntoEncodedString,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded: FromEncodedStr,
    <<SerdeLite<JsonSerdeCodec> as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`SerdeLite`] encoding.
    pub fn new_serde_lite(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> SharedValue<T, JsonSerdeWasmCodec>
where
    JsonSerdeWasmCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeWasmCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeWasmCodec as Decoder<T>>::Error: Debug,
    <JsonSerdeWasmCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeWasmCodec as Decoder<T>>::Encoded: FromEncodedStr,
    <<JsonSerdeWasmCodec as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`JsonSerdeWasmCodec`] encoding.
    pub fn new_serde_wb(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "miniserde")]
impl<T> SharedValue<T, MiniserdeCodec>
where
    MiniserdeCodec: Encoder<T> + Decoder<T>,
    <MiniserdeCodec as Encoder<T>>::Error: Debug,
    <MiniserdeCodec as Decoder<T>>::Error: Debug,
    <MiniserdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <MiniserdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
    <<MiniserdeCodec as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`MiniserdeCodec`] encoding.
    pub fn new_miniserde(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "rkyv")]
impl<T> SharedValue<T, RkyvCodec>
where
    RkyvCodec: Encoder<T> + Decoder<T>,
    <RkyvCodec as Encoder<T>>::Error: Debug,
    <RkyvCodec as Decoder<T>>::Error: Debug,
    <RkyvCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <RkyvCodec as Decoder<T>>::Encoded: FromEncodedStr,
    <<RkyvCodec as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses the [`RkyvCodec`] encoding.
    pub fn new_rkyv(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

impl<T, Ser> SharedValue<T, Ser>
where
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
    <<Ser as codee::Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
{
    /// Wraps the initial value.
    ///
    /// If this is on the server, the function will be invoked and the value serialized. When it runs
    /// on the client, it will be deserialized without running the function again.
    ///
    /// This uses `Ser` as an encoding.
    pub fn new_with_encoding(initial: impl FnOnce() -> T) -> Self {
        let value: T;
        #[cfg(feature = "hydration")]
        {
            use reactive_graph::owner::Owner;
            use std::borrow::Borrow;

            let sc = Owner::current_shared_context();
            let id = sc.as_ref().map(|sc| sc.next_id()).unwrap_or_default();
            let serialized = sc.as_ref().and_then(|sc| sc.read_data(&id));
            let hydrating =
                sc.as_ref().map(|sc| sc.during_hydration()).unwrap_or(false);
            value = if hydrating {
                let value = match serialized {
                    None => {
                        #[cfg(feature = "tracing")]
                        tracing::error!("couldn't deserialize");
                        None
                    }
                    Some(data) => {
                        match <Ser as Decoder<T>>::Encoded::from_encoded_str(
                            &data,
                        ) {
                            #[allow(unused_variables)] // used in tracing
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!(
                                    "couldn't deserialize from {data:?}: {e:?}"
                                );
                                None
                            }
                            Ok(encoded) => {
                                let decoded = Ser::decode(encoded.borrow());
                                #[cfg(feature = "tracing")]
                                let decoded = decoded
                                    .inspect_err(|e| tracing::error!("{e:?}"));
                                decoded.ok()
                            }
                        }
                    }
                };
                value.unwrap_or_else(initial)
            } else {
                let init = initial();
                #[cfg(feature = "ssr")]
                if let Some(sc) = sc {
                    if sc.get_is_hydrating() {
                        match Ser::encode(&init)
                            .map(IntoEncodedString::into_encoded_string)
                        {
                            Ok(value) => sc.write_async(
                                id,
                                Box::pin(async move { value }),
                            ),
                            #[allow(unused_variables)] // used in tracing
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!("couldn't serialize: {e:?}");
                            }
                        }
                    }
                }
                init
            }
        }
        #[cfg(not(feature = "hydration"))]
        {
            value = initial();
        }
        Self {
            value,
            ser: PhantomData,
        }
    }
}

impl<T, Ser> Deref for SharedValue<T, Ser> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, Ser> DerefMut for SharedValue<T, Ser> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T, Ser> PartialEq for SharedValue<T, Ser>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T, Ser> Eq for SharedValue<T, Ser> where T: Eq {}

impl<T, Ser> Display for SharedValue<T, Ser>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T, Ser> Hash for SharedValue<T, Ser>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T, Ser> PartialOrd for SharedValue<T, Ser>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T, Ser> Ord for SharedValue<T, Ser>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}
