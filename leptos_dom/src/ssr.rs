#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

//! Server-side HTML rendering utilities.

use crate::{
    html::{ElementChildren, StringOrView},
    CoreComponent, HydrationCtx, HydrationKey, IntoView, View,
};
use cfg_if::cfg_if;
use futures::{stream::FuturesUnordered, Future, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::*;
use std::pin::Pin;

type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Renders the given function to a static HTML string.
///
/// ```
/// # cfg_if::cfg_if! { if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
/// # use leptos::*;
/// let html = leptos::ssr::render_to_string(|| view! {
///   <p>"Hello, world!"</p>
/// });
/// // trim off the beginning, which has a bunch of hydration info, for comparison
/// assert!(html.contains("Hello, world!</p>"));
/// # }}
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_string<F, N>(f: F) -> Oco<'static, str>
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    HydrationCtx::reset_id();
    let runtime = leptos_reactive::create_runtime();

    let html = f().into_view().render_to_string();

    runtime.dispose();

    html
}

/// Renders a function to a stream of HTML strings.
///
/// This renders:
/// 1) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 2) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 3) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_stream(
    view: impl FnOnce() -> View + 'static,
) -> impl Stream<Item = String> {
    render_to_stream_with_prefix(view, || "".into())
}

/// Renders a function to a stream of HTML strings. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same reactive graph.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_stream_with_prefix(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
) -> impl Stream<Item = String> {
    let (stream, runtime) =
        render_to_stream_with_prefix_undisposed(view, prefix);
    runtime.dispose();
    stream
}

/// Renders a function to a stream of HTML strings and returns the [RuntimeId] that was created, so
/// it can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same reactive graph.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_stream_with_prefix_undisposed(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
) -> (impl Stream<Item = String>, RuntimeId) {
    render_to_stream_with_prefix_undisposed_with_context(view, prefix, || {})
}

/// Renders a function to a stream of HTML strings and returns the [RuntimeId] that was created, so
/// they can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same reactive graph.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_stream_with_prefix_undisposed_with_context(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
    additional_context: impl FnOnce() + 'static,
) -> (impl Stream<Item = String>, RuntimeId) {
    render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
        view,
        prefix,
        additional_context,
        false,
    )
}

/// Renders a function to a stream of HTML strings and returns the [RuntimeId] that was created, so
/// they can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same reactive graph.
///
/// If `replace_blocks` is true, this will wait for any fragments with blocking resources and
/// actually replace them in the initial HTML. This is slower to render (as it requires walking
/// back over the HTML for string replacement) but has the advantage of never including those fallbacks
/// in the HTML.
///
/// This renders:
/// 1) the prefix
/// 2) the application shell
///   a) HTML for everything that is not under a `<Suspense/>`,
///   b) the `fallback` for any `<Suspense/>` component that is not already resolved, and
///   c) JavaScript necessary to receive streaming [Resource](leptos_reactive::Resource) data.
/// 3) streaming [Resource](leptos_reactive::Resource) data. Resources begin loading on the
///    server and are sent down to the browser to resolve. On the browser, if the app sees that
///    it is waiting for a resource to resolve from the server, it doesn't run it initially.
/// 4) HTML fragments to replace each `<Suspense/>` fallback with its actual data as the resources
///    read under that `<Suspense/>` resolve.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub fn render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
    additional_context: impl FnOnce() + 'static,
    replace_blocks: bool,
) -> (impl Stream<Item = String>, RuntimeId) {
    HydrationCtx::reset_id();

    // create the runtime
    let runtime = create_runtime();

    // Add additional context items
    additional_context();

    // the actual app body/template code
    // this does NOT contain any of the data being loaded asynchronously in resources
    let shell = view().render_to_string();

    let resources = SharedContext::pending_resources();
    let pending_resources = serde_json::to_string(&resources).unwrap();
    let pending_fragments = SharedContext::pending_fragments();
    let serializers = SharedContext::serialization_resolvers();
    let nonce_str = crate::nonce::use_nonce()
        .map(|nonce| format!(" nonce=\"{nonce}\""))
        .unwrap_or_default();

    let local_only = SharedContext::fragments_with_local_resources();
    let local_only = serde_json::to_string(&local_only).unwrap();

    let mut blocking_fragments = FuturesUnordered::new();
    let fragments = FuturesUnordered::new();

    for (fragment_id, data) in pending_fragments {
        if data.should_block {
            blocking_fragments
                .push(async move { (fragment_id, data.out_of_order.await) });
        } else {
            fragments.push(Box::pin(async move {
                (fragment_id, data.out_of_order.await)
            })
                as Pin<Box<dyn Future<Output = (String, String)>>>);
        }
    }

    let stream = futures::stream::once(
        // HTML for the view function and script to store resources
        {
            let nonce_str = nonce_str.clone();
            async move {
                let resolvers = format!(
                    "<script{nonce_str}>__LEPTOS_PENDING_RESOURCES = \
                     {pending_resources};__LEPTOS_RESOLVED_RESOURCES = new \
                     Map();__LEPTOS_RESOURCE_RESOLVERS = new \
                     Map();__LEPTOS_LOCAL_ONLY = {local_only};</script>"
                );

                if replace_blocks {
                    let mut blocks =
                        Vec::with_capacity(blocking_fragments.len());
                    while let Some((blocked_id, blocked_fragment)) =
                        blocking_fragments.next().await
                    {
                        blocks.push((blocked_id, blocked_fragment));
                    }

                    let prefix = prefix();

                    let mut shell = shell;

                    for (blocked_id, blocked_fragment) in blocks {
                        let open = format!("<!--suspense-open-{blocked_id}-->");
                        let close =
                            format!("<!--suspense-close-{blocked_id}-->");
                        let (first, rest) =
                            shell.split_once(&open).unwrap_or_default();
                        let (_fallback, rest) =
                            rest.split_once(&close).unwrap_or_default();

                        shell =
                            format!("{first}{blocked_fragment}{rest}").into();
                    }

                    format!("{prefix}{shell}{resolvers}")
                } else {
                    let mut blocking = String::new();
                    let mut blocking_fragments = fragments_to_chunks(
                        nonce_str.clone(),
                        blocking_fragments,
                    );

                    while let Some(fragment) = blocking_fragments.next().await {
                        blocking.push_str(&fragment);
                    }
                    let prefix = prefix();
                    format!("{prefix}{shell}{resolvers}{blocking}")
                }
            }
        },
    )
    .chain(ooo_body_stream_recurse(nonce_str, fragments, serializers));

    (stream, runtime)
}

fn ooo_body_stream_recurse(
    nonce_str: String,
    fragments: FuturesUnordered<PinnedFuture<(String, String)>>,
    serializers: FuturesUnordered<PinnedFuture<(ResourceId, String)>>,
) -> Pin<Box<dyn Stream<Item = String>>> {
    // resources and fragments
    // stream HTML for each <Suspense/> as it resolves
    let fragments = fragments_to_chunks(nonce_str.clone(), fragments);
    // stream data for each Resource as it resolves
    let resources = render_serializers(nonce_str.clone(), serializers);

    Box::pin(
        // TODO these should be combined again in a way that chains them appropriately
        // such that individual resources can resolve before all fragments are done
        fragments.chain(resources).chain(
            futures::stream::once(async move {
                let pending = SharedContext::pending_fragments();

                if !pending.is_empty() {
                    let fragments = FuturesUnordered::new();
                    let serializers = SharedContext::serialization_resolvers();
                    for (fragment_id, data) in pending {
                        fragments.push(Box::pin(async move {
                            (fragment_id.clone(), data.out_of_order.await)
                        })
                            as Pin<Box<dyn Future<Output = (String, String)>>>);
                    }
                    Box::pin(ooo_body_stream_recurse(
                        nonce_str,
                        fragments,
                        serializers,
                    ))
                        as Pin<Box<dyn Stream<Item = String>>>
                } else {
                    Box::pin(futures::stream::once(async move {
                        Default::default()
                    }))
                }
            })
            .flatten(),
        ),
    )
}

#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
fn fragments_to_chunks(
    nonce_str: String,
    fragments: impl Stream<Item = (String, String)>,
) -> impl Stream<Item = String> {
    fragments.map(move |(fragment_id, html)| {
      format!(
        r#"
                <template id="{fragment_id}f">{html}</template>
                <script{nonce_str}>
                    (function() {{ let id = "{fragment_id}";
                    let open = undefined;
                    let close = undefined;
                    let walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
                    while(walker.nextNode()) {{
                         if(walker.currentNode.textContent == `suspense-open-${{id}}`) {{
                           open = walker.currentNode;
                         }} else if(walker.currentNode.textContent == `suspense-close-${{id}}`) {{
                           close = walker.currentNode;
                         }}
                      }}
                    let range = new Range();
                    range.setStartAfter(open);
                    range.setEndBefore(close);
                    range.deleteContents();
                    let tpl = document.getElementById("{fragment_id}f");
                    close.parentNode.insertBefore(tpl.content.cloneNode(true), close);}})()
                </script>
                "#
      )
    })
}

impl View {
    /// Consumes the node and renders it into an HTML string.
    ///
    /// This is __NOT__ the same as [`render_to_string`]. This
    /// functions differs in that it assumes a runtime is in scope.
    /// [`render_to_string`] creates, and disposes of a runtime for you.
    ///
    /// # Panics
    /// When called in a scope without a runtime. Use [`render_to_string`] instead.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn render_to_string(self) -> Oco<'static, str> {
        #[cfg(all(feature = "web", feature = "ssr"))]
        crate::logging::console_error(
            "\n[DANGER] You have both `csr` and `ssr` or `hydrate` and `ssr` \
             enabled as features, which may cause issues like <Suspense/>` \
             failing to work silently.\n",
        );

        self.render_to_string_helper(false)
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub(crate) fn render_to_string_helper(
        self,
        dont_escape_text: bool,
    ) -> Oco<'static, str> {
        match self {
            View::Text(node) => {
                if dont_escape_text {
                    node.content
                } else {
                    html_escape::encode_safe(&node.content).to_string().into()
                }
            }
            View::Component(node) => {
                let content = || {
                    node.children
                        .into_iter()
                        .map(|node| {
                            node.render_to_string_helper(dont_escape_text)
                        })
                        .join("")
                };
                cfg_if! {
                  if #[cfg(debug_assertions)] {
                    let name = to_kebab_case(&node.name);
                    let content = format!(r#"{}{}{}"#,
                      node.id.to_marker(false, &name),
                      content(),
                      node.id.to_marker(true, &name),
                    );
                    if let Some(id) = node.view_marker {
                        format!("<!--leptos-view|{id}|open-->{content}<!--leptos-view|{id}|close-->").into()
                    } else {
                        content.into()
                    }
                  } else {
                    format!(
                      r#"{}{}"#,
                      content(),
                      node.id.to_marker(true)
                    ).into()
                  }
                }
            }
            View::Suspense(id, node) => format!(
                "<!--suspense-open-{id}-->{}<!--suspense-close-{id}-->",
                View::CoreComponent(node)
                    .render_to_string_helper(dont_escape_text)
            )
            .into(),
            View::CoreComponent(node) => {
                let (id, name, wrap, content) = match node {
                    CoreComponent::Unit(u) => (
                        u.id,
                        "",
                        false,
                        Box::new(move || {
                            u.id.to_marker(
                                true,
                                #[cfg(debug_assertions)]
                                "unit",
                            )
                        })
                            as Box<dyn FnOnce() -> Oco<'static, str>>,
                    ),
                    CoreComponent::DynChild(node) => {
                        let child = node.child.take();
                        (
                            node.id,
                            "dyn-child",
                            true,
                            Box::new(move || {
                                if let Some(child) = *child {
                                    if let View::Text(t) = child {
                                        // if we don't check if the string is empty,
                                        // the HTML is an empty string; but an empty string
                                        // is not a text node in HTML, so can't be updated
                                        // in the future. so we put a one-space text node instead
                                        let was_empty = t.content.is_empty();
                                        let content = if was_empty {
                                            " ".into()
                                        } else {
                                            t.content
                                        };
                                        // escape content unless we're in a <script> or <style>
                                        let content = if dont_escape_text {
                                            content
                                        } else {
                                            html_escape::encode_safe(&content)
                                                .to_string()
                                                .into()
                                        };
                                        // On debug builds, `DynChild` has two marker nodes,
                                        // so there is no way for the text to be merged with
                                        // surrounding text when the browser parses the HTML,
                                        // but in release, `DynChild` only has a trailing marker,
                                        // and the browser automatically merges the dynamic text
                                        // into one single node, so we need to artificially make the
                                        // browser create the dynamic text as it's own text node
                                        if !cfg!(debug_assertions) {
                                            format!("<!>{content}",).into()
                                        } else {
                                            content
                                        }
                                    } else {
                                        child.render_to_string_helper(
                                            dont_escape_text,
                                        )
                                    }
                                } else {
                                    "".into()
                                }
                            })
                                as Box<dyn FnOnce() -> Oco<'static, str>>,
                        )
                    }
                    CoreComponent::Each(node) => {
                        let children = node.children.take();
                        (
                            node.id,
                            "each",
                            true,
                            Box::new(move || {
                                children
                                    .into_iter()
                                    .flatten()
                                    .map(|node| {
                                        let id = node.id;
                                        let is_el = matches!(
                                            node.child,
                                            View::Element(_)
                                        );

                                        let content = || {
                                            node.child.render_to_string_helper(
                                                dont_escape_text,
                                            )
                                        };

                                        if is_el {
                                            content()
                                        } else {
                                            format!(
                                                "{}{}{}",
                                                id.to_marker(
                                                    false,
                                                    #[cfg(debug_assertions)]
                                                    "each-item",
                                                ),
                                                content(),
                                                id.to_marker(
                                                    true,
                                                    #[cfg(debug_assertions)]
                                                    "each-item",
                                                )
                                            )
                                            .into()
                                        }
                                    })
                                    .join("")
                                    .into()
                            })
                                as Box<dyn FnOnce() -> Oco<'static, str>>,
                        )
                    }
                };

                if wrap {
                    format!(
                        r#"{}{}{}"#,
                        id.to_marker(
                            false,
                            #[cfg(debug_assertions)]
                            name,
                        ),
                        content(),
                        id.to_marker(
                            true,
                            #[cfg(debug_assertions)]
                            name,
                        ),
                    )
                    .into()
                } else {
                    content()
                }
            }
            View::Element(el) => {
                let is_script_or_style =
                    el.name == "script" || el.name == "style";
                let el_html = if let ElementChildren::Chunks(chunks) =
                    el.children
                {
                    chunks
                        .into_iter()
                        .map(|chunk| match chunk {
                            StringOrView::String(string) => string,
                            StringOrView::View(view) => view()
                                .render_to_string_helper(is_script_or_style),
                        })
                        .join("")
                        .into()
                } else {
                    let tag_name: Oco<'_, str> = el.name;

                    let mut inner_html: Option<Oco<'_, str>> = None;

                    let attrs = el
                        .attrs
                        .into_iter()
                        .filter_map(
                            |(name, value)| -> Option<Oco<'static, str>> {
                                if value.is_empty() {
                                    Some(format!(" {name}").into())
                                } else if name == "inner_html" {
                                    inner_html = Some(value);
                                    None
                                } else {
                                    Some(
                                        format!(
                                            " {name}=\"{}\"",
                                            html_escape::encode_double_quoted_attribute(&value)
                                        )
                                        .into(),
                                    )
                                }
                            },
                        )
                        .join("");

                    if el.is_void {
                        format!("<{tag_name}{attrs}/>").into()
                    } else if let Some(inner_html) = inner_html {
                        format!("<{tag_name}{attrs}>{inner_html}</{tag_name}>")
                            .into()
                    } else {
                        let children = match el.children {
                            ElementChildren::Empty => "".into(),
                            ElementChildren::Children(c) => c
                                .into_iter()
                                .map(|v| {
                                    v.render_to_string_helper(
                                        is_script_or_style,
                                    )
                                })
                                .join("")
                                .into(),
                            ElementChildren::InnerHtml(h) => h,
                            // already handled this case above
                            ElementChildren::Chunks(_) => unreachable!(),
                        };

                        format!("<{tag_name}{attrs}>{children}</{tag_name}>")
                            .into()
                    }
                };
                cfg_if! {
                    if #[cfg(debug_assertions)] {
                        if let Some(id) = el.view_marker {
                            format!("<!--leptos-view|{id}|open-->{el_html}<!--leptos-view|{id}|close-->").into()
                        } else {
                            el_html
                        }
                    } else {
                        el_html
                    }
                }
            }
            View::Transparent(_) => Default::default(),
        }
    }
}

#[cfg(debug_assertions)]
pub(crate) fn to_kebab_case(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    let mut new_name = String::with_capacity(name.len() + 8);

    let mut chars = name.chars();

    new_name.push(
        chars
            .next()
            .map(|mut c| {
                if c.is_ascii() {
                    c.make_ascii_lowercase();
                }

                c
            })
            .unwrap(),
    );

    for mut char in chars {
        if char.is_ascii_uppercase() {
            char.make_ascii_lowercase();

            new_name.push('-');
        }

        new_name.push(char);
    }

    new_name
}
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
pub(crate) fn render_serializers(
    nonce_str: String,
    serializers: FuturesUnordered<PinnedFuture<(ResourceId, String)>>,
) -> impl Stream<Item = String> {
    serializers.map(move |(id, json)| {
        let id = serde_json::to_string(&id).unwrap();
        let json = json.replace('<', "\\u003c");

        format!(
            r#"<script{nonce_str}>
                  (function() {{ let val = {json:?};
                  if(__LEPTOS_RESOURCE_RESOLVERS.get({id})) {{
                      __LEPTOS_RESOURCE_RESOLVERS.get({id})(val)
                  }} else {{
                      __LEPTOS_RESOLVED_RESOURCES.set({id}, val);
                  }} }})();
              </script>"#,
        )
    })
}

#[doc(hidden)]
pub fn escape_attr<T>(value: &T) -> Oco<'_, str>
where
    T: AsRef<str>,
{
    html_escape::encode_double_quoted_attribute(value).into()
}

pub(crate) trait ToMarker {
    fn to_marker(
        &self,
        closing: bool,
        #[cfg(debug_assertions)] component_name: &str,
    ) -> Oco<'static, str>;
}

impl ToMarker for HydrationKey {
    #[inline(always)]
    fn to_marker(
        &self,
        closing: bool,
        #[cfg(debug_assertions)] mut component_name: &str,
    ) -> Oco<'static, str> {
        #[cfg(debug_assertions)]
        {
            if component_name.is_empty() {
                // NOTE:
                // If the name is left empty, this will lead to invalid comments,
                // so a placeholder is used here.
                component_name = "<>";
            }
            if closing || component_name == "unit" {
                format!("<!--hk={self}c|leptos-{component_name}-end-->").into()
            } else {
                format!("<!--hk={self}o|leptos-{component_name}-start-->")
                    .into()
            }
        }
        #[cfg(not(debug_assertions))]
        {
            if closing {
                format!("<!--hk={self}-->").into()
            } else {
                "".into()
            }
        }
    }
}

impl ToMarker for Option<HydrationKey> {
    #[inline(always)]
    fn to_marker(
        &self,
        closing: bool,
        #[cfg(debug_assertions)] component_name: &str,
    ) -> Oco<'static, str> {
        self.map(|key| {
            key.to_marker(
                closing,
                #[cfg(debug_assertions)]
                component_name,
            )
        })
        .unwrap_or("".into())
    }
}
