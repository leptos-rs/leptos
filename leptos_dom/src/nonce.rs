use crate::{Attribute, IntoAttribute};
use leptos_reactive::{use_context, Scope};
use std::{fmt::Display, ops::Deref};

/// A nonce a cryptographic nonce ("number used once") which can be
/// used by Content Security Policy to determine whether or not a given
/// resource will be allowed to load.
///
/// When the `nonce` feature is enabled on one of the server integrations,
/// a nonce is generated during server rendering and added to all inline
/// scripts used for HTML streaming and resource loading.
///
/// The nonce being used during the current server response can be
/// accessed using [`use_nonce`](use_nonce).
///
/// /// ```rust,ignore
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     let csp = use_nonce(cx).map(|nonce| {
///         view! { cx,
///             <Meta
///                 http_equiv="Content-Security-Policy"
///                 content=format!("script-src 'nonce-{nonce}' 'unsafe-eval'")
///              />
///         }
///     });
///
///     view! { cx,
///       {csp}
///       <script nonce=use_nonce(cx)>"console.log('Hello, world!');"</script>
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Nonce(pub(crate) String);

impl Deref for Nonce {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl IntoAttribute for Nonce {
    fn into_attribute(self, _cx: Scope) -> Attribute {
        Attribute::String(self.0.into())
    }

    fn into_attribute_boxed(self: Box<Self>, _cx: Scope) -> Attribute {
        Attribute::String(self.0.into())
    }
}

impl IntoAttribute for Option<Nonce> {
    fn into_attribute(self, cx: Scope) -> Attribute {
        Attribute::Option(cx, self.map(|n| n.0.into()))
    }

    fn into_attribute_boxed(self: Box<Self>, cx: Scope) -> Attribute {
        Attribute::Option(cx, self.map(|n| n.0.into()))
    }
}

/// Accesses the nonce that has been generated during the current
/// server response. This can be added to inline `<script>` and
/// `<style>` tags for compatibility with a Content Security Policy.
///
/// ```rust,ignore
/// #[component]
/// pub fn App(cx: Scope) -> impl IntoView {
///     let csp = use_nonce(cx).map(|nonce| {
///         view! { cx,
///             <Meta
///                 http_equiv="Content-Security-Policy"
///                 content=format!("script-src 'nonce-{nonce}' 'unsafe-eval'")
///              />
///         }
///     });
///
///     view! { cx,
///       {csp}
///       <script nonce=use_nonce(cx)>"console.log('Hello, world!');"</script>
///     }
/// }
/// ```
pub fn use_nonce(cx: Scope) -> Option<Nonce> {
    use_context::<Nonce>(cx)
}

#[cfg(feature = "nonce")]
pub use generate::*;

#[cfg(feature = "nonce")]
mod generate {
    use super::Nonce;
    use base64::{
        alphabet,
        engine::{self, general_purpose},
        Engine,
    };
    use leptos_reactive::{provide_context, Scope};
    use rand::{thread_rng, RngCore};

    const NONCE_ENGINE: engine::GeneralPurpose = engine::GeneralPurpose::new(
        &alphabet::URL_SAFE,
        general_purpose::NO_PAD,
    );

    #[cfg(all(feature = "ssr", feature = "nonce"))]
    impl Nonce {
        /// Generates a new nonce from 16 bytes (128 bits) of random data.
        pub fn new() -> Self {
            let mut thread_rng = thread_rng();
            let mut bytes = [0; 16];
            thread_rng.fill_bytes(&mut bytes);
            Nonce(NONCE_ENGINE.encode(bytes))
        }
    }

    impl Default for Nonce {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Generates a nonce and provides it during server rendering.
    pub fn provide_nonce(cx: Scope) {
        provide_context(cx, Nonce::new())
    }
}
