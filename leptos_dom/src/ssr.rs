#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

//! Server-side HTML rendering utilities.

use crate::{
    html::{ElementChildren, StringOrView},
    CoreComponent, HydrationCtx, IntoView, View,
};
use cfg_if::cfg_if;
use futures::{stream::FuturesUnordered, Future, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::*;
use std::{borrow::Cow, pin::Pin};

type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Renders the given function to a static HTML string.
///
/// ```
/// # cfg_if::cfg_if! { if #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
/// # use leptos::*;
/// let html = leptos::ssr::render_to_string(|cx| view! { cx,
///   <p>"Hello, world!"</p>
/// });
/// // trim off the beginning, which has a bunch of hydration info, for comparison
/// assert!(html.contains("Hello, world!</p>"));
/// # }}
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "info", skip_all,)
)]
pub fn render_to_string<F, N>(f: F) -> String
where
    F: FnOnce(Scope) -> N + 'static,
    N: IntoView,
{
    let runtime = leptos_reactive::create_runtime();
    HydrationCtx::reset_id();

    let html = leptos_reactive::run_scope(runtime, |cx| {
        f(cx).into_view(cx).render_to_string(cx)
    });

    runtime.dispose();

    html.into()
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
    instrument(level = "info", skip_all,)
)]
pub fn render_to_stream(
    view: impl FnOnce(Scope) -> View + 'static,
) -> impl Stream<Item = String> {
    render_to_stream_with_prefix(view, |_| "".into())
}

/// Renders a function to a stream of HTML strings. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same `Scope`.
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
    instrument(level = "info", skip_all,)
)]
pub fn render_to_stream_with_prefix(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> impl Stream<Item = String> {
    let (stream, runtime, _) =
        render_to_stream_with_prefix_undisposed(view, prefix);
    runtime.dispose();
    stream
}

/// Renders a function to a stream of HTML strings and returns the [Scope] and [RuntimeId] that were created, so
/// they can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same `Scope`.
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
    instrument(level = "info", skip_all,)
)]
pub fn render_to_stream_with_prefix_undisposed(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> (impl Stream<Item = String>, RuntimeId, ScopeId) {
    render_to_stream_with_prefix_undisposed_with_context(view, prefix, |_cx| {})
}

/// Renders a function to a stream of HTML strings and returns the [Scope] and [RuntimeId] that were created, so
/// they can be disposed when appropriate. After the `view` runs, the `prefix` will run with
/// the same scope. This can be used to generate additional HTML that has access to the same `Scope`.
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
    instrument(level = "info", skip_all,)
)]
pub fn render_to_stream_with_prefix_undisposed_with_context(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
    additional_context: impl FnOnce(Scope) + 'static,
) -> (impl Stream<Item = String>, RuntimeId, ScopeId) {
    HydrationCtx::reset_id();

    // create the runtime
    let runtime = create_runtime();

    let (
        (shell, pending_resources, pending_fragments, serializers),
        scope,
        disposer,
    ) = run_scope_undisposed(runtime, {
        move |cx| {
            // Add additional context items
            additional_context(cx);
            // the actual app body/template code
            // this does NOT contain any of the data being loaded asynchronously in resources
            let shell = view(cx).render_to_string(cx);

            let resources = cx.pending_resources();
            let pending_resources = serde_json::to_string(&resources).unwrap();

            (
                shell,
                pending_resources,
                cx.pending_fragments(),
                cx.serialization_resolvers(),
            )
        }
    });
    let cx = Scope { runtime, id: scope };

    let blocking_fragments = FuturesUnordered::new();
    let fragments = FuturesUnordered::new();

    for (fragment_id, data) in pending_fragments {
        if data.should_block {
            blocking_fragments
                .push(async move { (fragment_id, data.out_of_order.await) });
        } else {
            fragments
                .push(async move { (fragment_id, data.out_of_order.await) });
        }
    }

    // resources and fragments
    // stream HTML for each <Suspense/> as it resolves
    let fragments = fragments_to_chunks(fragments);
    // stream data for each Resource as it resolves
    let resources = render_serializers(serializers);

    // HTML for the view function and script to store resources
    let stream = futures::stream::once(async move {
        let mut blocking = String::new();
        let mut blocking_fragments = fragments_to_chunks(blocking_fragments);
        while let Some(fragment) = blocking_fragments.next().await {
            blocking.push_str(&fragment);
        }
        let prefix = prefix(cx);
        format!(
            r#"
                {prefix}
                {shell}
                <script>
                    __LEPTOS_PENDING_RESOURCES = {pending_resources};
                    __LEPTOS_RESOLVED_RESOURCES = new Map();
                    __LEPTOS_RESOURCE_RESOLVERS = new Map();
                </script>
                {blocking}
            "#
        )
    })
    // TODO these should be combined again in a way that chains them appropriately
    // such that individual resources can resolve before all fragments are done
    .chain(fragments)
    .chain(resources)
    // dispose of the root scope
    .chain(futures::stream::once(async move {
        disposer.dispose();
        Default::default()
    }));

    (stream, runtime, scope)
}
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    instrument(level = "trace", skip_all,)
)]
fn fragments_to_chunks(
    fragments: impl Stream<Item = (String, String)>,
) -> impl Stream<Item = String> {
    fragments.map(|(fragment_id, html)| {
      format!(
        r#"
                <template id="{fragment_id}f">{html}</template>
                <script>
                    var id = "{fragment_id}";
                    var open = undefined;
                    var close = undefined;
                    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
                    while(walker.nextNode()) {{
                         if(walker.currentNode.textContent == `suspense-open-${{id}}`) {{
                           open = walker.currentNode;
                         }} else if(walker.currentNode.textContent == `suspense-close-${{id}}`) {{
                           close = walker.currentNode;
                         }}
                      }}
                    var range = new Range();
                    range.setStartAfter(open);
                    range.setEndBefore(close);
                    range.deleteContents();
                    var tpl = document.getElementById("{fragment_id}f");
                    close.parentNode.insertBefore(tpl.content.cloneNode(true), close);
                </script>
                "#
      )
    })
}

impl View {
    /// Consumes the node and renders it into an HTML string.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", skip_all,)
    )]
    pub fn render_to_string(self, _cx: Scope) -> Cow<'static, str> {
        #[cfg(all(feature = "web", feature = "ssr"))]
        crate::console_error(
            "\n[DANGER] You have both `csr` and `ssr` or `hydrate` and `ssr` \
             enabled as features, which may cause issues like <Suspense/>` \
             failing to work silently. `csr` is enabled by default on \
             `leptos`, and can be disabled by adding `default-features = \
             false` to your `leptos` dependency.\n",
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
    ) -> Cow<'static, str> {
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
                    let content = format!(r#"<!--hk={}|leptos-{name}-start-->{}<!--hk={}|leptos-{name}-end-->"#,
                      HydrationCtx::to_string(&node.id, false),
                      content(),
                      HydrationCtx::to_string(&node.id, true),
                      name = to_kebab_case(&node.name)
                    );
                    if let Some(id) = node.view_marker {
                        format!("<!--leptos-view|{id}|open-->{content}<!--leptos-view|{id}|close-->").into()
                    } else {
                        content.into()
                    }
                  } else {
                    format!(
                      r#"{}<!--hk={}-->"#,
                      content(),
                      HydrationCtx::to_string(&node.id, true)
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
                        u.id.clone(),
                        "",
                        false,
                        Box::new(move || {
                            #[cfg(debug_assertions)]
                            {
                                format!(
                                    "<!--hk={}|leptos-unit-->",
                                    HydrationCtx::to_string(&u.id, true)
                                )
                                .into()
                            }

                            #[cfg(not(debug_assertions))]
                            format!(
                                "<!--hk={}-->",
                                HydrationCtx::to_string(&u.id, true)
                            )
                            .into()
                        })
                            as Box<dyn FnOnce() -> Cow<'static, str>>,
                    ),
                    CoreComponent::DynChild(node) => {
                        let child = node.child.take();
                        (
                            node.id,
                            "dyn-child",
                            true,
                            Box::new(move || {
                                if let Some(child) = *child {
                                    // On debug builds, `DynChild` has two marker nodes,
                                    // so there is no way for the text to be merged with
                                    // surrounding text when the browser parses the HTML,
                                    // but in release, `DynChild` only has a trailing marker,
                                    // and the browser automatically merges the dynamic text
                                    // into one single node, so we need to artificially make the
                                    // browser create the dynamic text as it's own text node
                                    if let View::Text(t) = child {
                                        if !cfg!(debug_assertions) {
                                            format!("<!>{}", t.content).into()
                                        } else {
                                            t.content
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
                                as Box<dyn FnOnce() -> Cow<'static, str>>,
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

                                        let content = || {
                                            node.child.render_to_string_helper(
                                                dont_escape_text,
                                            )
                                        };

                                        #[cfg(debug_assertions)]
                                        {
                                            format!(
                        "<!--hk={}|leptos-each-item-start-->{}<!\
                         --hk={}|leptos-each-item-end-->",
                        HydrationCtx::to_string(&id, false),
                        content(),
                        HydrationCtx::to_string(&id, true),
                      )
                                        }

                                        #[cfg(not(debug_assertions))]
                                        format!(
                                            "{}<!--hk={}-->",
                                            content(),
                                            HydrationCtx::to_string(&id, true)
                                        )
                                    })
                                    .join("")
                                    .into()
                            })
                                as Box<dyn FnOnce() -> Cow<'static, str>>,
                        )
                    }
                };

                if wrap {
                    cfg_if! {
                      if #[cfg(debug_assertions)] {
                        format!(
                          r#"<!--hk={}|leptos-{name}-start-->{}<!--hk={}|leptos-{name}-end-->"#,
                          HydrationCtx::to_string(&id, false),
                          content(),
                          HydrationCtx::to_string(&id, true),
                        ).into()
                      } else {
                        let _ = name;

                        format!(
                          r#"{}<!--hk={}-->"#,
                          content(),
                          HydrationCtx::to_string(&id, true)
                        ).into()
                      }
                    }
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
                    let tag_name = el.name;

                    let mut inner_html = None;

                    let attrs = el
                        .attrs
                        .into_iter()
                        .filter_map(
                            |(name, value)| -> Option<Cow<'static, str>> {
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
    serializers: FuturesUnordered<PinnedFuture<(ResourceId, String)>>,
) -> impl Stream<Item = String> {
    serializers.map(|(id, json)| {
        let id = serde_json::to_string(&id).unwrap();
        let json = json.replace('<', "\\u003c");
        format!(
            r#"<script>
                  var val = {json:?};
                  if(__LEPTOS_RESOURCE_RESOLVERS.get({id})) {{
                      __LEPTOS_RESOURCE_RESOLVERS.get({id})(val)
                  }} else {{
                      __LEPTOS_RESOLVED_RESOURCES.set({id}, val);
                  }}
              </script>"#,
        )
    })
}

#[doc(hidden)]
pub fn escape_attr<T>(value: &T) -> Cow<'_, str>
where
    T: AsRef<str>,
{
    html_escape::encode_double_quoted_attribute(value)
}
