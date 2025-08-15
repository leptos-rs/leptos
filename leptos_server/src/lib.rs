//! Utilities for communicating between the server and the client with Leptos.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod action;
pub use action::*;
use std::borrow::Borrow;
mod local_resource;
pub use local_resource::*;
mod multi_action;
pub use multi_action::*;
mod once_resource;
pub use once_resource::*;
mod resource;
pub use resource::*;
mod shared;

use base64::{engine::general_purpose::STANDARD_NO_PAD, DecodeError, Engine};
/// Re-export of the `codee` crate.
pub use codee;
pub use shared::*;

/// Encodes data into a string.
pub trait IntoEncodedString {
    /// Encodes the data.
    fn into_encoded_string(self) -> String;
}

/// Decodes data from a string.
pub trait FromEncodedStr {
    /// The decoded data.
    type DecodedType<'a>: Borrow<Self>;

    /// The type of an error encountered during decoding.
    type DecodingError;

    /// Decodes the string.
    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError>;
}

impl IntoEncodedString for String {
    fn into_encoded_string(self) -> String {
        self
    }
}

impl FromEncodedStr for str {
    type DecodedType<'a> = &'a str;
    type DecodingError = ();

    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError> {
        Ok(data)
    }
}

impl IntoEncodedString for Vec<u8> {
    fn into_encoded_string(self) -> String {
        STANDARD_NO_PAD.encode(self)
    }
}

impl FromEncodedStr for [u8] {
    type DecodedType<'a> = Vec<u8>;
    type DecodingError = DecodeError;

    fn from_encoded_str(
        data: &str,
    ) -> Result<Self::DecodedType<'_>, Self::DecodingError> {
        STANDARD_NO_PAD.decode(data)
    }
}

#[cfg(feature = "tachys")]
mod view_implementations {
    use crate::Resource;
    use reactive_graph::traits::Read;
    use std::future::Future;
    use tachys::{
        html::attribute::{any_attribute::AnyAttribute, Attribute},
        hydration::Cursor,
        reactive_graph::{RenderEffectState, Suspend, SuspendState},
        ssr::StreamBuilder,
        view::{
            add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml,
        },
    };

    impl<T, Ser> Render for Resource<T, Ser>
    where
        T: Render + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type State = RenderEffectState<SuspendState<T>>;

        fn build(self) -> Self::State {
            (move || Suspend::new(async move { self.await })).build()
        }

        fn rebuild(self, state: &mut Self::State) {
            (move || Suspend::new(async move { self.await })).rebuild(state)
        }
    }

    impl<T, Ser> AddAnyAttr for Resource<T, Ser>
    where
        T: RenderHtml + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type Output<SomeNewAttr: Attribute> = Box<
            dyn FnMut() -> Suspend<
                <T as AddAnyAttr>::Output<
                    <SomeNewAttr::CloneableOwned as Attribute>::CloneableOwned,
                >,
            >
            + Send
        >;

        fn add_any_attr<NewAttr: Attribute>(
            self,
            attr: NewAttr,
        ) -> Self::Output<NewAttr>
        where
            Self::Output<NewAttr>: RenderHtml,
        {
            (move || Suspend::new(async move { self.await })).add_any_attr(attr)
        }
    }

    impl<T, Ser> RenderHtml for Resource<T, Ser>
    where
        T: RenderHtml + Send + Sync + Clone,
        Ser: Send + 'static,
    {
        type AsyncOutput = Option<T>;
        type Owned = Self;

        const MIN_LENGTH: usize = 0;

        fn dry_resolve(&mut self) {
            self.read();
        }

        fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send {
            (move || Suspend::new(async move { self.await })).resolve()
        }

        fn to_html_with_buf(
            self,
            buf: &mut String,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
            extra_attrs: Vec<AnyAttribute>,
        ) {
            (move || Suspend::new(async move { self.await })).to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            );
        }

        fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
            self,
            buf: &mut StreamBuilder,
            position: &mut Position,
            escape: bool,
            mark_branches: bool,
            extra_attrs: Vec<AnyAttribute>,
        ) where
            Self: Sized,
        {
            (move || Suspend::new(async move { self.await }))
                .to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                    extra_attrs,
                );
        }

        fn hydrate<const FROM_SERVER: bool>(
            self,
            cursor: &Cursor,
            position: &PositionState,
        ) -> Self::State {
            (move || Suspend::new(async move { self.await }))
                .hydrate::<FROM_SERVER>(cursor, position)
        }

        fn into_owned(self) -> Self::Owned {
            self
        }
    }
}
