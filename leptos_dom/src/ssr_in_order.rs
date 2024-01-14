#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

//! Server-side HTML rendering utilities for in-order streaming and async rendering.

use crate::{
    html::{ElementChildren, StringOrView},
    ssr::{render_serializers, ToMarker},
    CoreComponent, HydrationCtx, View,
};
use async_recursion::async_recursion;
use futures::{channel::mpsc::UnboundedSender, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::{
    create_runtime, suspense::StreamChunk, Oco, RuntimeId, SharedContext,
};
use std::collections::VecDeque;

/// Renders a view to HTML, waiting to return until all `async` [Resource](leptos_reactive::Resource)s
/// loaded in `<Suspense/>` elements have finished loading.
#[tracing::instrument(level = "trace", skip_all)]
pub async fn render_to_string_async(
    view: impl FnOnce() -> View + 'static,
) -> String {
    let mut buf = String::new();
    let (stream, runtime) =
        render_to_stream_in_order_with_prefix_undisposed_with_context(
            view,
            || "".into(),
            || {},
        );
    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.next().await {
        buf.push_str(&chunk);
    }
    runtime.dispose();
    buf
}

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components. The stream contains,
/// in order:
/// 1. HTML from the `view` in order, pausing to wait for each `<Suspense/>`
/// 2. any serialized [Resource](leptos_reactive::Resource)s
#[tracing::instrument(level = "trace", skip_all)]
pub fn render_to_stream_in_order(
    view: impl FnOnce() -> View + 'static,
) -> impl Stream<Item = String> {
    render_to_stream_in_order_with_prefix(view, || "".into())
}

/// Renders an in-order HTML stream, pausing at `<Suspense/>` components. The stream contains,
/// in order:
/// 1. `prefix`
/// 2. HTML from the `view` in order, pausing to wait for each `<Suspense/>`
/// 3. any serialized [Resource](leptos_reactive::Resource)s
///
/// `additional_context` is injected before the `view` is rendered. The `prefix` is generated
/// after the `view` is rendered, but before `<Suspense/>` nodes have resolved.
#[tracing::instrument(level = "trace", skip_all)]
pub fn render_to_stream_in_order_with_prefix(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
) -> impl Stream<Item = String> {
    #[cfg(all(feature = "web", feature = "ssr"))]
    crate::logging::console_error(
        "\n[DANGER] You have both `csr` and `ssr` or `hydrate` and `ssr` \
         enabled as features, which may cause issues like <Suspense/>` \
         failing to work silently.\n",
    );

    let (stream, runtime) =
        render_to_stream_in_order_with_prefix_undisposed_with_context(
            view,
            prefix,
            || {},
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
#[tracing::instrument(level = "trace", skip_all)]
pub fn render_to_stream_in_order_with_prefix_undisposed_with_context(
    view: impl FnOnce() -> View + 'static,
    prefix: impl FnOnce() -> Oco<'static, str> + 'static,
    additional_context: impl FnOnce() + 'static,
) -> (impl Stream<Item = String>, RuntimeId) {
    HydrationCtx::reset_id();

    // create the runtime
    let runtime = create_runtime();

    // add additional context
    additional_context();

    // render view and return chunks
    let view = view();

    let blocking_fragments_ready = SharedContext::blocking_fragments_ready();
    let chunks = view.into_stream_chunks();
    let pending_resources =
        serde_json::to_string(&SharedContext::pending_resources()).unwrap();

    let (tx, rx) = futures::channel::mpsc::unbounded();
    let (prefix_tx, prefix_rx) = futures::channel::oneshot::channel();
    leptos_reactive::spawn_local(async move {
        blocking_fragments_ready.await;

        let remaining_chunks = handle_blocking_chunks(tx.clone(), chunks).await;

        let prefix = prefix();
        prefix_tx.send(prefix).expect("to send prefix");
        handle_chunks(tx, remaining_chunks).await;
    });

    let nonce = crate::nonce::use_nonce();
    let nonce_str = nonce
        .as_ref()
        .map(|nonce| format!(" nonce=\"{nonce}\""))
        .unwrap_or_default();

    let local_only = SharedContext::fragments_with_local_resources();
    let local_only = serde_json::to_string(&local_only).unwrap();

    let stream = futures::stream::once({
        let nonce_str = nonce_str.clone();
        async move {
            let prefix = prefix_rx.await.expect("to receive prefix");
            format!(
                r#"
        {prefix}
        <script{nonce_str}>
            __LEPTOS_PENDING_RESOURCES = {pending_resources};
            __LEPTOS_RESOLVED_RESOURCES = new Map();
            __LEPTOS_RESOURCE_RESOLVERS = new Map();
            __LEPTOS_LOCAL_ONLY = {local_only};
        </script>
      "#
            )
        }
    })
    .chain(rx)
    .chain(
        futures::stream::once(async move {
            let serializers = SharedContext::serialization_resolvers();
            render_serializers(nonce_str, serializers)
        })
        .flatten(),
    );

    (stream, runtime)
}

#[tracing::instrument(level = "trace", skip_all)]
#[async_recursion(?Send)]
async fn handle_blocking_chunks(
    tx: UnboundedSender<String>,
    mut queued_chunks: VecDeque<StreamChunk>,
) -> VecDeque<StreamChunk> {
    let mut buffer = String::new();
    while let Some(chunk) = queued_chunks.pop_front() {
        match chunk {
            StreamChunk::Sync(sync) => buffer.push_str(&sync),
            StreamChunk::Async {
                chunks,
                should_block,
            } => {
                if should_block {
                    // add static HTML before the Suspense and stream it down
                    tx.unbounded_send(std::mem::take(&mut buffer))
                        .expect("failed to send async HTML chunk");

                    // send the inner stream
                    let suspended = chunks.await;
                    handle_blocking_chunks(tx.clone(), suspended).await;
                } else {
                    // TODO: should probably first check if there are any *other* blocking chunks
                    queued_chunks.push_front(StreamChunk::Async {
                        chunks,
                        should_block: false,
                    });
                    break;
                }
            }
        }
    }

    // send final sync chunk
    tx.unbounded_send(std::mem::take(&mut buffer))
        .expect("failed to send final HTML chunk");

    queued_chunks
}

#[tracing::instrument(level = "trace", skip_all)]
#[async_recursion(?Send)]
async fn handle_chunks(
    tx: UnboundedSender<String>,
    chunks: VecDeque<StreamChunk>,
) {
    let mut buffer = String::new();
    for chunk in chunks {
        match chunk {
            StreamChunk::Sync(sync) => buffer.push_str(&sync),
            StreamChunk::Async { chunks, .. } => {
                // add static HTML before the Suspense and stream it down
                tx.unbounded_send(std::mem::take(&mut buffer))
                    .expect("failed to send async HTML chunk");

                // send the inner stream

                let suspended = chunks.await;

                handle_chunks(tx.clone(), suspended).await;
            }
        }
    }
    // send final sync chunk
    tx.unbounded_send(std::mem::take(&mut buffer))
        .expect("failed to send final HTML chunk");
}

impl View {
    /// Renders the view into a set of HTML chunks that can be streamed.
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn into_stream_chunks(self) -> VecDeque<StreamChunk> {
        let mut chunks = VecDeque::new();
        self.into_stream_chunks_helper(&mut chunks, false);
        chunks
    }
    #[tracing::instrument(level = "trace", skip_all)]
    fn into_stream_chunks_helper(
        self,
        chunks: &mut VecDeque<StreamChunk>,
        dont_escape_text: bool,
    ) {
        match self {
            View::Suspense(id, view) => {
                let id = id.to_string();
                if let Some(data) = SharedContext::take_pending_fragment(&id) {
                    chunks.push_back(StreamChunk::Async {
                        chunks: data.in_order,
                        should_block: data.should_block,
                    });
                } else {
                    // if not registered, means it was already resolved
                    View::CoreComponent(view)
                        .into_stream_chunks_helper(chunks, dont_escape_text);
                }
            }
            View::Text(node) => {
                chunks.push_back(StreamChunk::Sync(node.content))
            }
            View::Component(node) => {
                #[cfg(debug_assertions)]
                let name = crate::ssr::to_kebab_case(&node.name);

                if cfg!(debug_assertions) {
                    chunks.push_back(StreamChunk::Sync(node.id.to_marker(
                        false,
                        #[cfg(debug_assertions)]
                        &name,
                    )));
                }

                for child in node.children {
                    child.into_stream_chunks_helper(chunks, dont_escape_text);
                }
                chunks.push_back(StreamChunk::Sync(node.id.to_marker(
                    true,
                    #[cfg(debug_assertions)]
                    &name,
                )));
            }
            View::Element(el) => {
                let is_script_or_style =
                    el.name == "script" || el.name == "style";

                #[cfg(debug_assertions)]
                if let Some(id) = &el.view_marker {
                    chunks.push_back(StreamChunk::Sync(
                        format!("<!--leptos-view|{id}|open-->").into(),
                    ));
                }
                if let ElementChildren::Chunks(el_chunks) = el.children {
                    for chunk in el_chunks {
                        match chunk {
                            StringOrView::String(string) => {
                                chunks.push_back(StreamChunk::Sync(string))
                            }
                            StringOrView::View(view) => view()
                                .into_stream_chunks_helper(
                                    chunks,
                                    is_script_or_style,
                                ),
                        }
                    }
                } else {
                    let tag_name = el.name;

                    let mut inner_html = None;

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
                        chunks.push_back(StreamChunk::Sync(
                            format!("<{tag_name}{attrs}/>").into(),
                        ));
                    } else if let Some(inner_html) = inner_html {
                        chunks.push_back(StreamChunk::Sync(
                            format!(
                                "<{tag_name}{attrs}>{inner_html}</{tag_name}>"
                            )
                            .into(),
                        ));
                    } else {
                        chunks.push_back(StreamChunk::Sync(
                            format!("<{tag_name}{attrs}>").into(),
                        ));

                        match el.children {
                            ElementChildren::Empty => {}
                            ElementChildren::Children(children) => {
                                for child in children {
                                    child.into_stream_chunks_helper(
                                        chunks,
                                        is_script_or_style,
                                    );
                                }
                            }
                            ElementChildren::InnerHtml(inner_html) => {
                                chunks.push_back(StreamChunk::Sync(inner_html))
                            }
                            // handled above
                            ElementChildren::Chunks(_) => unreachable!(),
                        }

                        chunks.push_back(StreamChunk::Sync(
                            format!("</{tag_name}>").into(),
                        ));
                    }
                }
                #[cfg(debug_assertions)]
                if let Some(id) = &el.view_marker {
                    chunks.push_back(StreamChunk::Sync(
                        format!("<!--leptos-view|{id}|close-->").into(),
                    ));
                }
            }
            View::Transparent(_) => {}
            View::CoreComponent(node) => {
                let (id, name, wrap, content) = match node {
                    CoreComponent::Unit(u) => (
                        u.id,
                        "",
                        false,
                        Box::new(move |chunks: &mut VecDeque<StreamChunk>| {
                            chunks.push_back(StreamChunk::Sync(
                                u.id.to_marker(
                                    true,
                                    #[cfg(debug_assertions)]
                                    "unit",
                                ),
                            ));
                        })
                            as Box<dyn FnOnce(&mut VecDeque<StreamChunk>)>,
                    ),
                    CoreComponent::DynChild(node) => {
                        let child = node.child.take();
                        (
                            node.id,
                            "dyn-child",
                            true,
                            Box::new(
                                move |chunks: &mut VecDeque<StreamChunk>| {
                                    if let Some(child) = *child {
                                        if let View::Text(t) = child {
                                            // if we don't check if the string is empty,
                                            // the HTML is an empty string; but an empty string
                                            // is not a text node in HTML, so can't be updated
                                            // in the future. so we put a one-space text node instead
                                            let was_empty =
                                                t.content.is_empty();
                                            let content = if was_empty {
                                                " ".into()
                                            } else {
                                                t.content
                                            };
                                            // escape content unless we're in a <script> or <style>
                                            let content = if dont_escape_text {
                                                content
                                            } else {
                                                html_escape::encode_safe(
                                                    &content,
                                                )
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
                                            chunks.push_back(
                                                if !cfg!(debug_assertions) {
                                                    StreamChunk::Sync(
                                                        format!(
                                                            "<!>{}",
                                                            html_escape::encode_safe(
                                                                &content
                                                            )
                                                        )
                                                        .into(),
                                                    )
                                                } else {
                                                    StreamChunk::Sync(html_escape::encode_safe(
                                                        &content
                                                    ).to_string().into())
                                                },
                                            );
                                        } else {
                                            child.into_stream_chunks_helper(
                                                chunks,
                                                dont_escape_text,
                                            );
                                        }
                                    }
                                },
                            )
                                as Box<dyn FnOnce(&mut VecDeque<StreamChunk>)>,
                        )
                    }
                    CoreComponent::Each(node) => {
                        let children = node.children.take();
                        (
                            node.id,
                            "each",
                            true,
                            Box::new(
                                move |chunks: &mut VecDeque<StreamChunk>| {
                                    for node in children.into_iter().flatten() {
                                        let id = node.id;
                                        let is_el = matches!(
                                            node.child,
                                            View::Element(_)
                                        );

                                        #[cfg(debug_assertions)]
                                        if !is_el {
                                            chunks.push_back(StreamChunk::Sync(
                                                id.to_marker(
                                                    false,
                                                    "each-item",
                                                ),
                                            ))
                                        };
                                        node.child.into_stream_chunks_helper(
                                            chunks,
                                            dont_escape_text,
                                        );

                                        if !is_el {
                                            chunks.push_back(
                                                StreamChunk::Sync(
                                                    id.to_marker(
                                                        true,
                                                        #[cfg(
                                                            debug_assertions
                                                        )]
                                                        "each-item",
                                                    ),
                                                ),
                                            );
                                        }
                                    }
                                },
                            )
                                as Box<dyn FnOnce(&mut VecDeque<StreamChunk>)>,
                        )
                    }
                };

                if wrap {
                    #[cfg(debug_assertions)]
                    {
                        chunks.push_back(StreamChunk::Sync(
                            id.to_marker(false, name),
                        ));
                    }
                    content(chunks);
                    chunks.push_back(StreamChunk::Sync(id.to_marker(
                        true,
                        #[cfg(debug_assertions)]
                        name,
                    )));
                } else {
                    content(chunks);
                }
            }
        }
    }
}
