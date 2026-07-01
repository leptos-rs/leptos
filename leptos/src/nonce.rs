use std::{fmt::Display, ops::Deref, sync::Arc};
use tachys::html::attribute::AttributeValue;

macro_rules! nonce_type {
    ($($inner:tt)+) => {
        /// A cryptographic nonce ("number used once") which can be
        /// used by Content Security Policy to determine whether or not a given
        /// resource will be allowed to load.
        ///
        /// When the `nonce` feature is enabled on one of the server integrations,
        /// a nonce is generated during server rendering and added to all inline
        /// scripts used for HTML streaming and resource loading.
        ///
        /// The nonce being used during the current server response can be
        /// accessed using [`use_nonce`].
        ///
        /// ```rust,ignore
        /// #[component]
        /// pub fn App() -> impl IntoView {
        ///     provide_meta_context;
        ///
        ///     view! {
        ///         // use `leptos_meta` to insert a <meta> tag with the CSP
        ///         <Meta
        ///             http_equiv="Content-Security-Policy"
        ///             content=move || {
        ///                 // this will insert the CSP with nonce on the server, be empty on client
        ///                 use_nonce()
        ///                     .map(|nonce| {
        ///                         format!(
        ///                             "default-src 'self'; script-src 'strict-dynamic' 'nonce-{nonce}' \
        ///                             'wasm-unsafe-eval'; style-src 'nonce-{nonce}';"
        ///                         )
        ///                     })
        ///                     .unwrap_or_default()
        ///             }
        ///         />
        ///         // manually insert nonce during SSR on inline script
        ///         <script nonce=use_nonce()>"console.log('Hello, world!');"</script>
        ///         // leptos_meta <Style/> and <Script/> automatically insert the nonce
        ///         <Style>"body { color: blue; }"</Style>
        ///         <p>"Test"</p>
        ///     }
        /// }
        /// ```
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct Nonce($($inner)+);
    }
}

#[cfg(feature = "nonce")]
nonce_type! {pub(crate) Arc<str>}
#[cfg(not(feature = "nonce"))]
nonce_type! {()}

macro_rules! if_constructable {
    ($(($($unused_var:ident),*))? => $($rest:tt)*) => {
        #[cfg(feature = "nonce")]
        {
            $($rest)*
        }
        #[cfg(not(feature = "nonce"))]
        {
            $(_ = ($($unused_var),*);)?
            #[cfg(debug_assertions)]
            unreachable!("Nonce cannot be constructed without the \"nonce\" feature");
            #[cfg(not(debug_assertions))]
            unreachable!();
        }
    };
    ($($rest:tt)*) => {
        if_constructable!{ => $($rest)* }
    };
}

impl Nonce {
    /// Returns a reference to the inner reference-counted string slice representing the nonce.
    pub fn as_inner(&self) -> &Arc<str> {
        if_constructable! {
            &self.0
        }
    }
}

impl Deref for Nonce {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        if_constructable! {
            &self.0
        }
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if_constructable! { (f) =>
            write!(f, "{}", self.0)
        }
    }
}

impl AttributeValue for Nonce {
    type AsyncOutput = Self;
    #[cfg(feature = "nonce")]
    type State = <Arc<str> as AttributeValue>::State;
    #[cfg(not(feature = "nonce"))]
    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        if_constructable! {
            self.0.len()
        }
    }

    fn to_html(self, key: &str, buf: &mut String) {
        if_constructable! { (key, buf) =>
            <Arc<str> as AttributeValue>::to_html(self.0, key, buf)
        }
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &tachys::renderer::types::Element,
    ) -> Self::State {
        if_constructable! { (key, el) =>
            <Arc<str> as AttributeValue>::hydrate::<FROM_SERVER>(
                self.0, key, el,
            )
        }
    }

    fn build(
        self,
        el: &tachys::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        if_constructable! { (el, key) =>
            <Arc<str> as AttributeValue>::build(self.0, el, key)
        }
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        if_constructable! { (key, state) =>
            <Arc<str> as AttributeValue>::rebuild(self.0, key, state)
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

/// Accesses the nonce that has been generated during the current
/// server response. This can be added to inline `<script>` and
/// `<style>` tags for compatibility with a Content Security Policy.
///
/// This function can be called without the `nonce` feature enabled,
/// in which case it will always return [`None::<Nonce>`].
///
/// ```rust,ignore
/// #[component]
/// pub fn App() -> impl IntoView {
///     provide_meta_context;
///
///     view! {
///         // use `leptos_meta` to insert a <meta> tag with the CSP
///         <Meta
///             http_equiv="Content-Security-Policy"
///             content=move || {
///                 // this will insert the CSP with nonce on the server, be empty on client
///                 use_nonce()
///                     .map(|nonce| {
///                         format!(
///                             "default-src 'self'; script-src 'strict-dynamic' 'nonce-{nonce}' \
///                             'wasm-unsafe-eval'; style-src 'nonce-{nonce}';"
///                         )
///                     })
///                     .unwrap_or_default()
///             }
///         />
///         // manually insert nonce during SSR on inline script
///         <script nonce=use_nonce()>"console.log('Hello, world!');"</script>
///         // leptos_meta <Style/> and <Script/> automatically insert the nonce
///         <Style>"body { color: blue; }"</Style>
///         <p>"Test"</p>
///     }
/// }
/// ```
#[inline(always)]
pub fn use_nonce() -> Option<Nonce> {
    #[cfg(feature = "nonce")]
    return crate::context::use_context::<Nonce>();
    #[cfg(not(feature = "nonce"))]
    return None;
}

/// Generates a nonce and provides it via context.
#[cfg(feature = "nonce")]
pub fn provide_nonce() {
    crate::context::provide_context(Nonce::new())
}

#[cfg(feature = "nonce")]
impl Nonce {
    /// Generates a new nonce from 16 bytes (128 bits) of random data.
    pub fn new() -> Self {
        use base64::{
            alphabet,
            engine::{self, general_purpose},
            Engine as _,
        };
        use rand::{rng, RngCore as _};
        const NONCE_ENGINE: engine::GeneralPurpose =
            engine::GeneralPurpose::new(
                &alphabet::URL_SAFE,
                general_purpose::NO_PAD,
            );

        let mut rng = rng();
        let mut bytes = [0; 16];
        rng.fill_bytes(&mut bytes);
        Nonce(NONCE_ENGINE.encode(bytes).into())
    }

    /// Builds a nonce from a caller-supplied value rather than generating
    /// one — e.g. a nonce minted by a reverse proxy and forwarded to the
    /// application as a request header. The caller is responsible for the
    /// value's randomness and for it being a valid CSP nonce. Provide it via
    /// context before rendering so [`use_nonce`] and the hydration scripts
    /// pick it up.
    pub fn from_value(value: impl Into<Arc<str>>) -> Self {
        Nonce(value.into())
    }
}

#[cfg(feature = "nonce")]
impl Default for Nonce {
    fn default() -> Self {
        Self::new()
    }
}
