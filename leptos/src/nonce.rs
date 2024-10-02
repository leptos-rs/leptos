use crate::context::{provide_context, use_context};
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine,
};
use rand::{thread_rng, RngCore};
use std::{fmt::Display, ops::Deref, sync::Arc};
use tachys::html::attribute::AttributeValue;

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
pub struct Nonce(pub(crate) Arc<str>);

impl Deref for Nonce {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AttributeValue for Nonce {
    type AsyncOutput = Self;
    type State = <Arc<str> as AttributeValue>::State;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <Arc<str> as AttributeValue>::to_html(self.0, key, buf)
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &tachys::renderer::types::Element,
    ) -> Self::State {
        <Arc<str> as AttributeValue>::hydrate::<FROM_SERVER>(self.0, key, el)
    }

    fn build(
        self,
        el: &tachys::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        <Arc<str> as AttributeValue>::build(self.0, el, key)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        <Arc<str> as AttributeValue>::rebuild(self.0, key, state)
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
pub fn use_nonce() -> Option<Nonce> {
    use_context::<Nonce>()
}

/// Generates a nonce and provides it via context.
pub fn provide_nonce() {
    provide_context(Nonce::new())
}

const NONCE_ENGINE: engine::GeneralPurpose =
    engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

impl Nonce {
    /// Generates a new nonce from 16 bytes (128 bits) of random data.
    pub fn new() -> Self {
        let mut thread_rng = thread_rng();
        let mut bytes = [0; 16];
        thread_rng.fill_bytes(&mut bytes);
        Nonce(NONCE_ENGINE.encode(bytes).into())
    }
}

impl Default for Nonce {
    fn default() -> Self {
        Self::new()
    }
}
