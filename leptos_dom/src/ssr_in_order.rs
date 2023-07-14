#![cfg(not(all(target_arch = "wasm32", feature = "web")))]

//! Server-side HTML rendering utilities for in-order streaming and async rendering.

use crate::{
    html::{ElementChildren, StringOrView},
    ssr::render_serializers,
    CoreComponent, HydrationCtx, View,
};
use async_recursion::async_recursion;
use cfg_if::cfg_if;
use futures::{channel::mpsc::UnboundedSender, Stream, StreamExt};
use itertools::Itertools;
use leptos_reactive::{
    create_runtime, run_scope_undisposed, suspense::StreamChunk, RuntimeId,
    Scope, ScopeId,
};
use std::{borrow::Cow, collections::VecDeque};

/// Renders a view to HTML, waiting to return until all `async` [Resource](leptos_reactive::Resource)s
/// loaded in `<Suspense/>` elements have finished loading.
#[tracing::instrument(level = "info", skip_all)]
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
#[tracing::instrument(level = "info", skip_all)]
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
#[tracing::instrument(level = "trace", skip_all)]
pub fn render_to_stream_in_order_with_prefix(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
) -> impl Stream<Item = String> {
    #[cfg(all(feature = "web", feature = "ssr"))]
    crate::console_error(
        "\n[DANGER] You have both `csr` and `ssr` or `hydrate` and `ssr` \
         enabled as features, which may cause issues like <Suspense/>` \
         failing to work silently.\n",
    );

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
#[tracing::instrument(level = "trace", skip_all)]
pub fn render_to_stream_in_order_with_prefix_undisposed_with_context(
    view: impl FnOnce(Scope) -> View + 'static,
    prefix: impl FnOnce(Scope) -> Cow<'static, str> + 'static,
    additional_context: impl FnOnce(Scope) + 'static,
) -> (impl Stream<Item = String>, RuntimeId, ScopeId) {
    HydrationCtx::reset_id();

    // create the runtime
    let runtime = create_runtime();

    let (
        (blocking_fragments_ready, chunks, prefix, pending_resources),
        scope_id,
        _,
    ) = run_scope_undisposed(runtime, |cx| {
        // add additional context
        additional_context(cx);

        // render view and return chunks
        let view = view(cx);

        (
            cx.blocking_fragments_ready(),
            view.into_stream_chunks(cx),
            prefix,
            serde_json::to_string(&cx.pending_resources()).unwrap(),
        )
    });
    let cx = Scope {
        runtime,
        id: scope_id,
    };

    let (tx, rx) = futures::channel::mpsc::unbounded();
    let (prefix_tx, prefix_rx) = futures::channel::oneshot::channel();
    leptos_reactive::spawn_local(async move {
        blocking_fragments_ready.await;
        let remaining_chunks = handle_blocking_chunks(tx.clone(), chunks).await;
        let prefix = prefix(cx);
        prefix_tx.send(prefix).expect("to send prefix");
        handle_chunks(cx, tx, remaining_chunks).await;
    });

    let nonce = crate::nonce::use_nonce(cx);
    let nonce_str = nonce
        .as_ref()
        .map(|nonce| format!(" nonce=\"{nonce}\""))
        .unwrap_or_default();

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
        </script>
      "#
            )
        }
    })
    .chain(rx)
    .chain(
        futures::stream::once(async move {
            let serializers = cx.serialization_resolvers();
            render_serializers(nonce_str, serializers)
        })
        .flatten(),
    );

    (stream, runtime, scope_id)
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
    cx: Scope,
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
                handle_chunks(cx, tx.clone(), suspended).await;
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
    pub fn into_stream_chunks(self, cx: Scope) -> VecDeque<StreamChunk> {
        let mut chunks = VecDeque::new();
        self.into_stream_chunks_helper(cx, &mut chunks, false);
        chunks
    }
    #[tracing::instrument(level = "trace", skip_all)]
    fn into_stream_chunks_helper(
        self,
        cx: Scope,
        chunks: &mut VecDeque<StreamChunk>,
        dont_escape_text: bool,
    ) {
        match self {
            View::Suspense(id, view) => {
                let id = id.to_string();
                if let Some(data) = cx.take_pending_fragment(&id) {
                    chunks.push_back(StreamChunk::Async {
                        chunks: data.in_order,
                        should_block: data.should_block,
                    });
                } else {
                    // if not registered, means it was already resolved
                    View::CoreComponent(view).into_stream_chunks_helper(
                        cx,
                        chunks,
                        dont_escape_text,
                    );
                }
            }
            View::Text(node) => {
                chunks.push_back(StreamChunk::Sync(node.content))
            }
            View::Component(node) => {
                cfg_if! {
                  if #[cfg(debug_assertions)] {
                    let name = crate::ssr::to_kebab_case(&node.name);
                    chunks.push_back(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-start-->"#, HydrationCtx::to_string(&node.id, false)).into()));
                    for child in node.children {
                        child.into_stream_chunks_helper(cx, chunks, dont_escape_text);
                    }
                    chunks.push_back(StreamChunk::Sync(format!(r#"<!--hk={}|leptos-{name}-end-->"#, HydrationCtx::to_string(&node.id, true)).into()));
                  } else {
                    for child in node.children {
                        child.into_stream_chunks_helper(cx, chunks, dont_escape_text);
                    }
                    chunks.push_back(StreamChunk::Sync(format!(r#"<!--hk={}-->"#, HydrationCtx::to_string(&node.id, true)).into()))
                  }
                }
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
                            StringOrView::View(view) => {
                                view().into_stream_chunks_helper(
                                    cx,
                                    chunks,
                                    is_script_or_style,
                                );
                            }
                        }
                    }
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
                                        cx,
                                        chunks,
                                        is_script_or_style,
                                    );
                                }
                            }
                            ElementChildren::InnerHtml(inner_html) => {
                                chunks.push_back(StreamChunk::Sync(inner_html));
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
                            #[cfg(debug_assertions)]
                            {
                                chunks.push_back(StreamChunk::Sync(
                                    format!(
                                        "<!--hk={}|leptos-unit-->",
                                        HydrationCtx::to_string(&u.id, true)
                                    )
                                    .into(),
                                ));
                            }

                            #[cfg(not(debug_assertions))]
                            chunks.push_back(StreamChunk::Sync(
                                format!(
                                    "<!--hk={}-->",
                                    HydrationCtx::to_string(&u.id, true)
                                )
                                .into(),
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
                                        // On debug builds, `DynChild` has two marker nodes,
                                        // so there is no way for the text to be merged with
                                        // surrounding text when the browser parses the HTML,
                                        // but in release, `DynChild` only has a trailing marker,
                                        // and the browser automatically merges the dynamic text
                                        // into one single node, so we need to artificially make the
                                        // browser create the dynamic text as it's own text node
                                        if let View::Text(t) = child {
                                            let content = if dont_escape_text {
                                                t.content
                                            } else {
                                                html_escape::encode_safe(
                                                    &t.content,
                                                )
                                                .to_string()
                                                .into()
                                            };
                                            chunks.push_back(
                                                if !cfg!(debug_assertions) {
                                                    StreamChunk::Sync(
                                                        format!(
                                                            "<!>{}",
                                                            content
                                                        )
                                                        .into(),
                                                    )
                                                } else {
                                                    StreamChunk::Sync(content)
                                                },
                                            );
                                        } else {
                                            child.into_stream_chunks_helper(
                                                cx,
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

                                        #[cfg(debug_assertions)]
                                        {
                                            chunks.push_back(
                                                StreamChunk::Sync(
                                                    format!(
                        "<!--hk={}|leptos-each-item-start-->",
                        HydrationCtx::to_string(&id, false)
                      )
                                                    .into(),
                                                ),
                                            );
                                            node.child
                                                .into_stream_chunks_helper(
                                                    cx,
                                                    chunks,
                                                    dont_escape_text,
                                                );
                                            chunks.push_back(
                                                StreamChunk::Sync(
                                                    format!(
                        "<!--hk={}|leptos-each-item-end-->",
                        HydrationCtx::to_string(&id, true)
                      )
                                                    .into(),
                                                ),
                                            );
                                        }
                                        #[cfg(not(debug_assertions))]
                                        {
                                            node.child
                                                .into_stream_chunks_helper(
                                                    cx,
                                                    chunks,
                                                    dont_escape_text,
                                                );
                                            chunks.push_back(
                                                StreamChunk::Sync(
                                                    format!(
                                                        "<!--hk={}-->",
                                                        HydrationCtx::to_string(
                                                            &id, true
                                                        )
                                                    )
                                                    .into(),
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
                    cfg_if! {
                      if #[cfg(debug_assertions)] {
                        chunks.push_back(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-start-->", HydrationCtx::to_string(&id, false)).into()));
                        content(chunks);
                        chunks.push_back(StreamChunk::Sync(format!("<!--hk={}|leptos-{name}-end-->", HydrationCtx::to_string(&id, true)).into()));
                      } else {
                        let _ = name;
                        content(chunks);
                        chunks.push_back(StreamChunk::Sync(format!("<!--hk={}-->", HydrationCtx::to_string(&id, true)).into()))
                      }
                    }
                } else {
                    content(chunks);
                }
            }
        }
    }
}
