use crate::view::{Position, RenderHtml};
use futures::Stream;
use std::{
    collections::VecDeque,
    fmt::{Debug, Write},
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

/// Manages streaming HTML rendering for the response to a single request.
#[derive(Default)]
pub struct StreamBuilder {
    pub(crate) sync_buf: String,
    pub(crate) chunks: VecDeque<StreamChunk>,
    pending: Option<ChunkFuture>,
    pending_ooo: VecDeque<PinnedFuture<OooChunk>>,
    id: Option<Vec<u16>>,
}

type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type ChunkFuture = PinnedFuture<VecDeque<StreamChunk>>;

impl StreamBuilder {
    /// Creates a new HTML stream.
    pub fn new(id: Option<Vec<u16>>) -> Self {
        Self::with_capacity(0, id)
    }

    /// Creates a new stream with a given capacity in the synchronous buffer and an identifier.
    pub fn with_capacity(capacity: usize, id: Option<Vec<u16>>) -> Self {
        Self {
            id,
            sync_buf: String::with_capacity(capacity),
            ..Default::default()
        }
    }

    /// Reserves additional space in the synchronous buffer.
    pub fn reserve(&mut self, additional: usize) {
        self.sync_buf.reserve(additional);
    }

    /// Pushes text into the synchronous buffer.
    pub fn push_sync(&mut self, string: &str) {
        self.sync_buf.push_str(string);
    }

    /// Pushes an async block into the stream.
    pub fn push_async(
        &mut self,
        fut: impl Future<Output = VecDeque<StreamChunk>> + Send + 'static,
    ) {
        // flush sync chunk
        let sync = mem::take(&mut self.sync_buf);
        if !sync.is_empty() {
            self.chunks.push_back(StreamChunk::Sync(sync));
        }
        self.chunks.push_back(StreamChunk::Async {
            chunks: Box::pin(fut) as PinnedFuture<VecDeque<StreamChunk>>,
        });
    }

    /// Mutates the synchronous buffer.
    pub fn with_buf(&mut self, fun: impl FnOnce(&mut String)) {
        fun(&mut self.sync_buf)
    }

    /// Takes all chunks currently available in the stream, including the synchronous buffer.
    pub fn take_chunks(&mut self) -> VecDeque<StreamChunk> {
        let sync = mem::take(&mut self.sync_buf);
        if !sync.is_empty() {
            self.chunks.push_back(StreamChunk::Sync(sync));
        }
        mem::take(&mut self.chunks)
    }

    /// Appends another stream to this one.
    pub fn append(&mut self, mut other: StreamBuilder) {
        self.chunks.append(&mut other.chunks);
        self.sync_buf.push_str(&other.sync_buf);
    }

    /// Completes the stream.
    pub fn finish(mut self) -> Self {
        let sync_buf_remaining = mem::take(&mut self.sync_buf);
        if sync_buf_remaining.is_empty() {
            return self;
        } else if let Some(StreamChunk::Sync(buf)) = self.chunks.back_mut() {
            buf.push_str(&sync_buf_remaining);
        } else {
            self.chunks.push_back(StreamChunk::Sync(sync_buf_remaining));
        }
        self
    }

    // Out-of-Order Streaming
    /// Pushes a fallback for out-of-order streaming.
    pub fn push_fallback<View>(
        &mut self,
        fallback: View,
        position: &mut Position,
        mark_branches: bool,
    ) where
        View: RenderHtml,
    {
        self.write_chunk_marker(true);
        fallback.to_html_with_buf(
            &mut self.sync_buf,
            position,
            true,
            mark_branches,
        );
        self.write_chunk_marker(false);
        *position = Position::NextChild;
    }

    /// Increments the chunk ID.
    pub fn next_id(&mut self) {
        if let Some(last) = self.id.as_mut().and_then(|ids| ids.last_mut()) {
            *last += 1;
        }
    }

    /// Returns the current ID.
    pub fn clone_id(&self) -> Option<Vec<u16>> {
        self.id.clone()
    }

    /// Returns an ID that is a child of the current one.
    pub fn child_id(&self) -> Option<Vec<u16>> {
        let mut child = self.id.clone();
        if let Some(child) = child.as_mut() {
            child.push(0);
        }
        child
    }

    /// Inserts a marker for the current out-of-order chunk.
    pub fn write_chunk_marker(&mut self, opening: bool) {
        if let Some(id) = &self.id {
            self.sync_buf.reserve(11 + (id.len() * 2));
            self.sync_buf.push_str("<!--s-");
            for piece in id {
                write!(&mut self.sync_buf, "{}-", piece).unwrap();
            }
            if opening {
                self.sync_buf.push_str("o-->");
            } else {
                self.sync_buf.push_str("c-->");
            }
        }
    }

    /// Injects an out-of-order chunk into the stream.
    pub fn push_async_out_of_order<View>(
        &mut self,
        view: impl Future<Output = Option<View>> + Send + 'static,
        position: &mut Position,
        mark_branches: bool,
    ) where
        View: RenderHtml,
    {
        let id = self.clone_id();
        // copy so it's not updated by additional iterations
        // i.e., restart in the same position we were at when we suspended
        let mut position = *position;

        self.chunks.push_back(StreamChunk::OutOfOrder {
            chunks: Box::pin(async move {
                let view = view.await;

                let mut subbuilder = StreamBuilder::new(id);
                let mut id = String::new();
                if let Some(ids) = &subbuilder.id {
                    for piece in ids {
                        write!(&mut id, "{}-", piece).unwrap();
                    }
                }
                if let Some(id) = subbuilder.id.as_mut() {
                    id.push(0);
                }
                let replace = view.is_some();
                if let Some(view) = view {
                    view.to_html_async_with_buf::<true>(
                        &mut subbuilder,
                        &mut position,
                        true,
                        mark_branches,
                    );
                }
                let chunks = subbuilder.finish().take_chunks();

                OooChunk {
                    id,
                    chunks,
                    replace,
                }
            }),
        });
    }
}

impl Debug for StreamBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamBuilderInner")
            .field("sync_buf", &self.sync_buf)
            .field("chunks", &self.chunks)
            .field("pending", &self.pending.is_some())
            .finish()
    }
}

/// A chunk of the HTML stream.
pub enum StreamChunk {
    /// Some synchronously-available HTML.
    Sync(String),
    /// The chunk can be rendered asynchronously in order.
    Async {
        /// A collection of in-order chunks.
        chunks: PinnedFuture<VecDeque<StreamChunk>>,
    },
    /// The chunk can be rendered asynchronously out of order.
    OutOfOrder {
        /// A collection of out-of-order chunks
        chunks: PinnedFuture<OooChunk>,
    },
}

/// A chunk of the out-of-order stream.
#[derive(Debug)]
pub struct OooChunk {
    id: String,
    chunks: VecDeque<StreamChunk>,
    replace: bool,
}

impl OooChunk {
    /// Pushes an opening `<template>` tag into the buffer.
    pub fn push_start(id: &str, buf: &mut String) {
        buf.push_str("<template id=\"");
        buf.push_str(id);
        buf.push('f');
        buf.push_str("\">");
    }

    /// Pushes a closing `</template>` and update script into the buffer.
    pub fn push_end(replace: bool, id: &str, buf: &mut String) {
        buf.push_str("</template>");

        // TODO nonce
        buf.push_str("<script");
        buf.push_str(r#">(function() { let id = ""#);
        buf.push_str(id);
        buf.push_str(
            "\";let open = undefined;let close = undefined;let walker = \
             document.createTreeWalker(document.body, \
             NodeFilter.SHOW_COMMENT);while(walker.nextNode()) \
             {if(walker.currentNode.textContent == `s-${id}o`){ \
             open=walker.currentNode; } else \
             if(walker.currentNode.textContent == `s-${id}c`) { close = \
             walker.currentNode;}}let range = new Range(); \
             range.setStartBefore(open); range.setEndBefore(close);",
        );
        if replace {
            buf.push_str(
                "range.deleteContents(); let tpl = \
                 document.getElementById(`${id}f`); \
                 close.parentNode.insertBefore(tpl.content.cloneNode(true), \
                 close);close.remove();",
            );
        } else {
            buf.push_str("close.remove();open.remove();");
        }
        buf.push_str("})()</script>");
    }
}

impl Debug for StreamChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync(arg0) => f.debug_tuple("Sync").field(arg0).finish(),
            Self::Async { .. } => {
                f.debug_struct("Async").finish_non_exhaustive()
            }
            Self::OutOfOrder { .. } => {
                f.debug_struct("OutOfOrder").finish_non_exhaustive()
            }
        }
    }
}

impl Stream for StreamBuilder {
    type Item = String;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut();
        let pending = this.pending.take();
        if let Some(mut pending) = pending {
            match pending.as_mut().poll(cx) {
                Poll::Pending => {
                    this.pending = Some(pending);
                    Poll::Pending
                }
                Poll::Ready(chunks) => {
                    for chunk in chunks.into_iter().rev() {
                        this.chunks.push_front(chunk);
                    }
                    self.poll_next(cx)
                }
            }
        } else {
            let next_chunk = this.chunks.pop_front();
            match next_chunk {
                None => {
                    // now, handle out-of-order chunks
                    if let Some(mut pending) = this.pending_ooo.pop_front() {
                        match pending.as_mut().poll(cx) {
                            Poll::Ready(OooChunk {
                                id,
                                chunks,
                                replace,
                            }) => {
                                let opening = format!("<!--s-{id}o-->");
                                let placeholder_at =
                                    this.sync_buf.find(&opening);
                                if let Some(start) = placeholder_at {
                                    let closing = format!("<!--s-{id}c-->");
                                    let end =
                                        this.sync_buf.find(&closing).unwrap();
                                    let chunks_iter = chunks.into_iter().rev();

                                    // TODO can probably make this more efficient
                                    let (before, replaced) =
                                        this.sync_buf.split_at(start);
                                    let (_, after) = replaced
                                        .split_at(end - start + closing.len());
                                    let mut buf = String::new();
                                    buf.push_str(before);

                                    let mut held_chunks = VecDeque::new();
                                    for chunk in chunks_iter {
                                        if let StreamChunk::Sync(ready) = chunk
                                        {
                                            buf.push_str(&ready);
                                        } else {
                                            held_chunks.push_front(chunk);
                                        }
                                    }
                                    buf.push_str(after);
                                    this.sync_buf = buf;
                                    for chunk in held_chunks {
                                        this.chunks.push_front(chunk);
                                    }
                                } else {
                                    OooChunk::push_start(
                                        &id,
                                        &mut this.sync_buf,
                                    );
                                    for chunk in chunks.into_iter().rev() {
                                        if let StreamChunk::Sync(ready) = chunk
                                        {
                                            this.sync_buf.push_str(&ready);
                                        } else {
                                            this.chunks.push_front(chunk);
                                        }
                                    }
                                    OooChunk::push_end(
                                        replace,
                                        &id,
                                        &mut this.sync_buf,
                                    );
                                }
                                self.poll_next(cx)
                            }
                            Poll::Pending => {
                                this.pending_ooo.push_back(pending);
                                if this.sync_buf.is_empty() {
                                    Poll::Pending
                                } else {
                                    Poll::Ready(Some(mem::take(
                                        &mut this.sync_buf,
                                    )))
                                }
                            }
                        }
                    } else if this.sync_buf.is_empty() {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(mem::take(&mut this.sync_buf)))
                    }
                }
                Some(StreamChunk::Sync(value)) => {
                    this.sync_buf.push_str(&value);
                    loop {
                        match this.chunks.pop_front() {
                            None => break,
                            Some(StreamChunk::Async { chunks }) => {
                                this.chunks
                                    .push_front(StreamChunk::Async { chunks });
                                break;
                            }
                            Some(StreamChunk::OutOfOrder {
                                chunks, ..
                            }) => {
                                this.pending_ooo.push_back(chunks);
                                break;
                            }
                            Some(StreamChunk::Sync(next)) => {
                                this.sync_buf.push_str(&next);
                            }
                        }
                    }

                    this.poll_next(cx)
                }
                Some(StreamChunk::Async { chunks, .. }) => {
                    this.pending = Some(chunks);
                    if this.sync_buf.is_empty() {
                        self.poll_next(cx)
                    } else {
                        Poll::Ready(Some(mem::take(&mut this.sync_buf)))
                    }
                }
                Some(StreamChunk::OutOfOrder { chunks, .. }) => {
                    this.pending_ooo.push_back(chunks);
                    if this.sync_buf.is_empty() {
                        self.poll_next(cx)
                    } else {
                        Poll::Ready(Some(mem::take(&mut this.sync_buf)))
                    }
                }
            }
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use crate::{
        async_views::{FutureViewExt, Suspend},
        html::element::{em, main, p, ElementChild, HtmlElement, Main},
        renderer::dom::Dom,
        view::RenderHtml,
    };
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn in_order_stream_of_sync_content_ready_immediately() {
        let el: HtmlElement<Main, _, _, Dom> = main().child(p().child((
            "Hello, ",
            em().child("beautiful"),
            " world!",
        )));
        let mut stream = el.to_html_stream_in_order();

        let html = stream.next().await.unwrap();
        assert_eq!(
            html,
            "<main><p>Hello, <em>beautiful</em> world!</p></main>"
        );
    }

    #[tokio::test]
    async fn in_order_single_async_block_in_stream() {
        let el = async {
            sleep(Duration::from_millis(250)).await;
            "Suspended"
        }
        .suspend();
        let mut stream =
            <Suspend<false, _, _> as RenderHtml<Dom>>::to_html_stream_in_order(
                el,
            );

        let html = stream.next().await.unwrap();
        assert_eq!(html, "Suspended<!>");
    }

    #[tokio::test]
    async fn in_order_async_with_siblings_in_stream() {
        let el = (
            "Before Suspense",
            async {
                sleep(Duration::from_millis(250)).await;
                "Suspended"
            }
            .suspend(),
        );
        let mut stream =
            <(&str, Suspend<false, _, _>) as RenderHtml<Dom>>::to_html_stream_in_order(
                el,
            );

        assert_eq!(stream.next().await.unwrap(), "Before Suspense");
        assert_eq!(stream.next().await.unwrap(), "<!>Suspended");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn in_order_async_inside_element_in_stream() {
        let el: HtmlElement<_, _, _, Dom> = p().child((
            "Before Suspense",
            async {
                sleep(Duration::from_millis(250)).await;
                "Suspended"
            }
            .suspend(),
        ));
        let mut stream = el.to_html_stream_in_order();

        assert_eq!(stream.next().await.unwrap(), "<p>Before Suspense");
        assert_eq!(stream.next().await.unwrap(), "<!>Suspended</p>");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn in_order_nested_async_blocks() {
        let el: HtmlElement<_, _, _, Dom> = main().child((
            "Before Suspense",
            async {
                sleep(Duration::from_millis(250)).await;
                p().child((
                    "Before inner Suspense",
                    async {
                        sleep(Duration::from_millis(250)).await;
                        "Inner Suspense"
                    }
                    .suspend(),
                ))
            }
            .suspend(),
        ));
        let mut stream = el.to_html_stream_in_order();

        assert_eq!(stream.next().await.unwrap(), "<main>Before Suspense");
        assert_eq!(stream.next().await.unwrap(), "<p>Before inner Suspense");
        assert_eq!(
            stream.next().await.unwrap(),
            "<!>Inner Suspense</p></main>"
        );
    }

    #[tokio::test]
    async fn out_of_order_stream_of_sync_content_ready_immediately() {
        let el: HtmlElement<Main, _, _, Dom> = main().child(p().child((
            "Hello, ",
            em().child("beautiful"),
            " world!",
        )));
        let mut stream = el.to_html_stream_out_of_order();

        let html = stream.next().await.unwrap();
        assert_eq!(
            html,
            "<main><p>Hello, <em>beautiful</em> world!</p></main>"
        );
    }

    #[tokio::test]
    async fn out_of_order_single_async_block_in_stream() {
        let el = async {
            sleep(Duration::from_millis(250)).await;
            "Suspended"
        }
        .suspend()
        .with_fallback("Loading...");
        let mut stream =
            <Suspend<false, _, _> as RenderHtml<Dom>>::to_html_stream_out_of_order(
                el,
            );

        assert_eq!(
            stream.next().await.unwrap(),
            "<!--s-1-o-->Loading...<!--s-1-c-->"
        );
        assert_eq!(
            stream.next().await.unwrap(),
            "<template id=\"1-f\">Suspended</template><script>(function() { \
             let id = \"1-\";let open = undefined;let close = undefined;let \
             walker = document.createTreeWalker(document.body, \
             NodeFilter.SHOW_COMMENT);while(walker.nextNode()) \
             {if(walker.currentNode.textContent == `s-${id}o`){ \
             open=walker.currentNode; } else \
             if(walker.currentNode.textContent == `s-${id}c`) { close = \
             walker.currentNode;}}let range = new Range(); \
             range.setStartAfter(open); range.setEndBefore(close); \
             range.deleteContents(); let tpl = \
             document.getElementById(`${id}f`); \
             close.parentNode.insertBefore(tpl.content.cloneNode(true), \
             close);})()</script>"
        );
    }

    #[tokio::test]
    async fn out_of_order_inside_element_in_stream() {
        let el: HtmlElement<_, _, _, Dom> = p().child((
            "Before Suspense",
            async {
                sleep(Duration::from_millis(250)).await;
                "Suspended"
            }
            .suspend()
            .with_fallback("Loading..."),
            "After Suspense",
        ));
        let mut stream = el.to_html_stream_out_of_order();

        assert_eq!(
            stream.next().await.unwrap(),
            "<p>Before Suspense<!--s-1-o--><!>Loading...<!--s-1-c-->After \
             Suspense</p>"
        );
        assert!(stream.next().await.unwrap().contains("Suspended"));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn out_of_order_nested_async_blocks() {
        let el: HtmlElement<_, _, _, Dom> = main().child((
            "Before Suspense",
            async {
                sleep(Duration::from_millis(250)).await;
                p().child((
                    "Before inner Suspense",
                    async {
                        sleep(Duration::from_millis(250)).await;
                        "Inner Suspense"
                    }
                    .suspend()
                    .with_fallback("Loading Inner..."),
                    "After inner Suspense",
                ))
            }
            .suspend()
            .with_fallback("Loading..."),
            "After Suspense",
        ));
        let mut stream = el.to_html_stream_out_of_order();

        assert_eq!(
            stream.next().await.unwrap(),
            "<main>Before Suspense<!--s-1-o--><!>Loading...<!--s-1-c-->After \
             Suspense</main>"
        );
        let loading_inner = stream.next().await.unwrap();
        assert!(loading_inner.contains(
            "<p>Before inner Suspense<!--s-1-1-o--><!>Loading \
             Inner...<!--s-1-1-c-->After inner Suspense</p>"
        ));
        assert!(loading_inner.contains("let id = \"1-\";"));

        let inner = stream.next().await.unwrap();
        assert!(inner.contains("Inner Suspense"));
        assert!(inner.contains("let id = \"1-1-\";"));

        assert!(stream.next().await.is_none());
    }
}
*/
