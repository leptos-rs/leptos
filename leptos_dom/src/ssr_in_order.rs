#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

//! Server-side HTML rendering utilities for in-order streaming and async rendering.

use crate::{ssr::render_serializers, CoreComponent, HydrationCtx, View};
use async_recursion::async_recursion;
use cfg_if::cfg_if;
use futures::{channel::mpsc::Sender, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::{
    create_runtime, run_scope_undisposed, suspense::StreamChunk, RuntimeId,
    Scope, ScopeId,
};
use std::borrow::Cow;

/// Renders a view to HTML, waiting to return until all `async` [Resource](leptos_reactive::Resource)s
/// loaded in `<Suspense/>` elements have finished loading.
pub async fn render_to_string_async(
    view: impl FnOnce(Scope) -> View + 'static,
) -> String {
    let mut buf = String::new();
    let mut stream = Box::pin(render_to_stream_in_order(view));
    while let Some(chunk) = stream.next().await {
        buf.push_str(&chunk);
    }
    buf
}

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components. The stream contains,
/// in order:
/// 1. HTML from the `view` in order, pausing to wait for each `<Suspense/>`
/// 2. any serialized [Resource](leptos_reactive::Resource)s
pub fn render_to_stream_in_order(
    view: impl FnOnce(Scope) -> View + 'static,
) -> impl Stream<Item = String> {
    render_to_stream_in_order_with_prefix(view, |_| "".into())
}

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components. The stream contains,
/// in order:
/// 1. `prefix`
/// 2. HTML from the `view` in order, pausing to wait for each `<Suspense/>`
/// 3. any serialized [Resource](leptos_reactive::Resource)s
///
/// `additional_context` is injected before the `view` is rendered. The `prefix` is generated
/// after the `view` is rendered, but before `<Suspense/>` nodes have resolved.
pub fn render_to_stream_in_order_with_prefix(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> impl Stream<Item = String> {
    let (stream, runtime, _) =
        render_to_stream_in_order_with_prefix_undisposed_with_context(
            view,
            prefix,
            |_| {},
        );
    runtime.dispose();
    stream
}

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components. The stream contains,
/// in order:
/// 1. `prefix`
/// 2. HTML from the `view` in order, pausing to wait for each `<Suspense/>`
/// 3. any serialized [Resource](leptos_reactive::Resource)s
///
/// `additional_context` is injected before the `view` is rendered. The `prefix` is generated
/// after the `view` is rendered, but before `<Suspense/>` nodes have resolved.
pub fn render_to_stream_in_order_with_prefix_undisposed_with_context(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
    additional_context: impl FnOnce(Scope) + 'static,
) -> (impl Stream<Item = String>, RuntimeId, ScopeId) {
    HydrationCtx::reset_id();

    // create the runtime
    let runtime = create_runtime();

    let ((chunks, prefix, pending_resources, serializers), scope_id, disposer) =
        run_scope_undisposed(runtime, |cx| {
            // add additional context
            additional_context(cx);

            // render view and return chunks
            let view = view(cx);

            let prefix = prefix(cx);
            (
                view.into_stream_chunks(cx),
                prefix,
                serde_json::to_string(&cx.pending_resources()).unwrap(),
                cx.serialization_resolvers(),
            )
        });

    let (tx, rx) = futures::channel::mpsc::channel(1);
    leptos_reactive::spawn_local(async move {
        handle_chunks(tx, chunks).await;
    });

    let stream = futures::stream::once(async move {
        format!(
            r#"
        {prefix}
        <script>
            __LEPTOS_PENDING_RESOURCES = {pending_resources};
            __LEPTOS_RESOLVED_RESOURCES = new Map();
            __LEPTOS_RESOURCE_RESOLVERS = new Map();
        </script>
      "#
        )
    })
    .chain(rx)
    .chain(render_serializers(serializers))
    // dispose of the scope
    .chain(futures::stream::once(async move {
        disposer.dispose();
        Default::default()
    }));

    (stream, runtime, scope_id)
}

#[async_recursion(?Send)]
async fn handle_chunks(mut tx: Sender<String>, chunks: Vec<StreamChunk>) {
    let mut buffer = String::new();
    for chunk in chunks {
        match chunk {
            StreamChunk::Sync(sync) => buffer.push_str(&sync),
            StreamChunk::Async(suspended) => {
                // add static HTML before the Suspense and stream it down
                _ = tx.try_send(std::mem::take(&mut buffer));

                // send the inner stream
                let suspended = suspended.await;
                handle_chunks(tx.clone(), suspended).await;
            }
        }
    }
    // send final sync chunk
    _ = tx.try_send(std::mem::take(&mut buffer));
}

impl View {
    /// Renders the view into a set of HTML chunks that can be streamed.
    pub fn into_stream_chunks(self, cx: Scope) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        self.into_stream_chunks_helper(cx, &mut chunks);
        chunks
    }

    fn into_stream_chunks_helper(
        self,
        cx: Scope,
        chunks: &mut Vec<StreamChunk>,
    ) {
        match self {
            View::Suspense(id, _) => {
                let id = id.to_string();
                if let Some((_, fragment)) = cx.take_pending_fragment(&id) {
                    chunks.push(StreamChunk::Async(fragment));
                }
            }
            View::Text(node) => chunks.push(StreamChunk::Sync(node.content)),
            View::Component(node) => {
                cfg_if! {
                  if #[cfg(debug_assertions)] {
                    let name = crate::ssr::to_kebab_case(&node.name);
                    chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-start-->"#, HydrationCtx::to_string(&node.id, false)).into()));
                    for child in node.children {
                        child.into_stream_chunks_helper(cx, chunks);
                    }
                    chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-end-->"#, HydrationCtx::to_string(&node.id, true)).into()));
                  } else {
                    for child in node.children {
                        child.into_stream_chunks_helper(cx, chunks);
                    }
                    chunks.push(StreamChunk::Sync(format!(r#"<!--hk={}-->"#, HydrationCtx::to_string(&node.id, true)).into()))
                  }
                }
            }
            View::Element(el) => {
                #[cfg(debug_assertions)]
                if let Some(id) = &el.view_marker {
                    chunks.push(StreamChunk::Sync(
                        format!("<!--leptos-view|{id}|open-->").into(),
                    ));
                }
                if let Some(prerendered) = el.prerendered {
                    chunks.push(StreamChunk::Sync(prerendered))
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
                        chunks.push(StreamChunk::Sync(
                            format!("<{tag_name}{attrs}/>").into(),
                        ));
                    } else if let Some(inner_html) = inner_html {
                        chunks.push(StreamChunk::Sync(
                            format!(
                                "<{tag_name}{attrs}>{inner_html}</{tag_name}>"
                            )
                            .into(),
                        ));
                    } else {
                        chunks.push(StreamChunk::Sync(
                            format!("<{tag_name}{attrs}>").into(),
                        ));
                        for child in el.children {
                            child.into_stream_chunks_helper(cx, chunks);
                        }

                        chunks.push(StreamChunk::Sync(
                            format!("</{tag_name}>").into(),
                        ));
                    }
                }
                #[cfg(debug_assertions)]
                if let Some(id) = &el.view_marker {
                    chunks.push(StreamChunk::Sync(
                        format!("<!--leptos-view|{id}|close-->").into(),
                    ));
                }
            }
            View::Transparent(_) => {}
            View::CoreComponent(node) => {
                let (id, name, wrap, content) = match node {
                    CoreComponent::Unit(u) => (
                        u.id.clone(),
                        "",
                        false,
                        Box::new(move |chunks: &mut Vec<StreamChunk>| {
                            #[cfg(debug_assertions)]
                            {
                                chunks.push(StreamChunk::Sync(
                                    format!(
                                        "<!--hk={}|leptos-unit-->",
                                        HydrationCtx::to_string(&u.id, true)
                                    )
                                    .into(),
                                ));
                            }

                            #[cfg(not(debug_assertions))]
                            chunks.push(StreamChunk::Sync(
                                format!(
                                    "<!--hk={}-->",
                                    HydrationCtx::to_string(&u.id, true)
                                )
                                .into(),
                            ));
                        })
                            as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
                    ),
                    CoreComponent::DynChild(node) => {
                        let child = node.child.take();
                        (
                            node.id,
                            "dyn-child",
                            true,
                            Box::new(move |chunks: &mut Vec<StreamChunk>| {
                                if let Some(child) = *child {
                                    // On debug builds, `DynChild` has two marker nodes,
                                    // so there is no way for the text to be merged with
                                    // surrounding text when the browser parses the HTML,
                                    // but in release, `DynChild` only has a trailing marker,
                                    // and the browser automatically merges the dynamic text
                                    // into one single node, so we need to artificially make the
                                    // browser create the dynamic text as it's own text node
                                    if let View::Text(t) = child {
                                        chunks.push(
                                            if !cfg!(debug_assertions) {
                                                StreamChunk::Sync(
                                                    format!("<!>{}", t.content)
                                                        .into(),
                                                )
                                            } else {
                                                StreamChunk::Sync(t.content)
                                            },
                                        );
                                    } else {
                                        child.into_stream_chunks_helper(
                                            cx, chunks,
                                        );
                                    }
                                }
                            })
                                as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
                        )
                    }
                    CoreComponent::Each(node) => {
                        let children = node.children.take();
                        (
                            node.id,
                            "each",
                            true,
                            Box::new(move |chunks: &mut Vec<StreamChunk>| {
                                for node in children.into_iter().flatten() {
                                    let id = node.id;

                                    #[cfg(debug_assertions)]
                                    {
                                        chunks.push(StreamChunk::Sync(
                                            format!(
                        "<!--hk={}|leptos-each-item-start-->",
                        HydrationCtx::to_string(&id, false)
                      )
                                            .into(),
                                        ));
                                        node.child.into_stream_chunks_helper(
                                            cx, chunks,
                                        );
                                        chunks.push(StreamChunk::Sync(
                                            format!(
                        "<!--hk={}|leptos-each-item-end-->",
                        HydrationCtx::to_string(&id, true)
                      )
                                            .into(),
                                        ));
                                    }
                                }
                            })
                                as Box<dyn FnOnce(&mut Vec<StreamChunk>)>,
                        )
                    }
                };

                if wrap {
                    cfg_if! {
                      if #[cfg(debug_assertions)] {
                        chunks.push(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-start-->", HydrationCtx::to_string(&id, false)).into()));
                        content(chunks);
                        chunks.push(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-end-->", HydrationCtx::to_string(&id, true)).into()));
                      } else {
                        let _ = name;
                        content(chunks);
                        chunks.push(StreamChunk::Sync(format!("<!--hk={}-->", HydrationCtx::to_string(&id, true)).into()))
                      }
                    }
                } else {
                    content(chunks);
                }
            }
        }
    }
}
