use super::{Encoding, FromReq, FromRes, IntoReq};
use crate::{
    ContentType, IntoRes, ServerFnError,
    error::{FromServerFnError, ServerFnErrorErr},
    request::{ClientReq, Req},
    response::{ClientRes, TryRes},
};
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use http::Method;
use std::{fmt::Debug, pin::Pin};

/// An encoding that represents a stream of bytes.
///
/// A server function that uses this as its output encoding should return [`ByteStream`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct Streaming;

impl ContentType for Streaming {
    const CONTENT_TYPE: &'static str = "application/octet-stream";
}

impl Encoding for Streaming {
    const METHOD: Method = Method::POST;
}

impl<E, T, Request> IntoReq<Streaming, Request, E> for T
where
    Request: ClientReq<E>,
    T: Stream<Item = Bytes> + Send + 'static,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        Request::try_new_post_streaming(
            path,
            accepts,
            Streaming::CONTENT_TYPE,
            self,
        )
    }
}

impl<E, T, Request> FromReq<Streaming, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: From<ByteStream<E>> + 'static,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_stream()?;
        let s = ByteStream::new(data.map_err(|e| E::de(e)));
        Ok(s.into())
    }
}

/// A stream of bytes.
///
/// A server function can return this type if its output encoding is [`Streaming`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct ByteStream<E = ServerFnError>(
    Pin<Box<dyn Stream<Item = Result<Bytes, E>> + Send>>,
);

impl<E> ByteStream<E> {
    /// Consumes the wrapper, returning a stream of bytes.
    pub fn into_inner(self) -> impl Stream<Item = Result<Bytes, E>> + Send {
        self.0
    }
}

impl<E> Debug for ByteStream<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ByteStream").finish()
    }
}

impl<E> ByteStream<E> {
    /// Creates a new `ByteStream` from the given stream.
    pub fn new<T>(
        value: impl Stream<Item = Result<T, E>> + Send + 'static,
    ) -> Self
    where
        T: Into<Bytes>,
    {
        Self(Box::pin(value.map(|value| value.map(Into::into))))
    }
}

impl<E, S, T> From<S> for ByteStream<E>
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<Bytes>,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

impl<E, Response> IntoRes<Streaming, Response, E> for ByteStream<E>
where
    Response: TryRes<E>,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        Response::try_from_stream(
            Streaming::CONTENT_TYPE,
            self.into_inner().map_err(|e| e.ser().body),
        )
    }
}

impl<E, Response> FromRes<Streaming, Response, E> for ByteStream<E>
where
    Response: ClientRes<E> + Send,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let stream = res.try_into_stream()?;
        Ok(ByteStream::new(stream.map_err(|e| E::de(e))))
    }
}

/// An encoding that represents a stream of text.
///
/// A server function that uses this as its output encoding should return [`TextStream`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct StreamingText;

impl ContentType for StreamingText {
    const CONTENT_TYPE: &'static str = "text/plain";
}

impl Encoding for StreamingText {
    const METHOD: Method = Method::POST;
}

/// A stream of text.
///
/// A server function can return this type if its output encoding is [`StreamingText`].
///
/// ## Browser Support for Streaming Input
///
/// Browser fetch requests do not currently support full request duplexing, which
/// means that that they do begin handling responses until the full request has been sent.
/// This means that if you use a streaming input encoding, the input stream needs to
/// end before the output will begin.
///
/// Streaming requests are only allowed over HTTP2 or HTTP3.
pub struct TextStream<E = ServerFnError>(
    Pin<Box<dyn Stream<Item = Result<String, E>> + Send>>,
);

impl<E> Debug for TextStream<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextStream").finish()
    }
}

impl<E> TextStream<E> {
    /// Creates a new `TextStream` from the given stream.
    pub fn new(
        value: impl Stream<Item = Result<String, E>> + Send + 'static,
    ) -> Self {
        Self(Box::pin(value.map(|value| value)))
    }
}

impl<E> TextStream<E> {
    /// Consumes the wrapper, returning a stream of text.
    pub fn into_inner(self) -> impl Stream<Item = Result<String, E>> + Send {
        self.0
    }
}

impl<E, S, T> From<S> for TextStream<E>
where
    S: Stream<Item = T> + Send + 'static,
    T: Into<String>,
{
    fn from(value: S) -> Self {
        Self(Box::pin(value.map(|data| Ok(data.into()))))
    }
}

fn utf8_error<E: FromServerFnError>(error: std::str::Utf8Error) -> E {
    E::from_server_fn_error(ServerFnErrorErr::Deserialization(
        error.to_string(),
    ))
}

fn decode_text_stream<E>(
    stream: impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static,
) -> impl Stream<Item = Result<String, E>> + Send
where
    E: FromServerFnError,
{
    futures::stream::unfold(
        (Box::pin(stream.fuse()), Vec::new()),
        |(mut stream, mut buffer)| async move {
            loop {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.extend_from_slice(&bytes);
                        match std::str::from_utf8(&buffer) {
                            Ok("") => continue,
                            Ok(text) => {
                                let text = text.to_owned();
                                buffer.clear();
                                return Some((Ok(text), (stream, buffer)));
                            }
                            Err(error) if error.error_len().is_none() => {
                                let valid_up_to = error.valid_up_to();
                                if valid_up_to == 0 {
                                    continue;
                                }
                                let text =
                                    std::str::from_utf8(&buffer[..valid_up_to])
                                        .map(str::to_owned)
                                        .map_err(utf8_error::<E>);
                                buffer = buffer.split_off(valid_up_to);
                                return Some((text, (stream, buffer)));
                            }
                            Err(error) => {
                                let error = utf8_error(error);
                                buffer.clear();
                                return Some((Err(error), (stream, buffer)));
                            }
                        }
                    }
                    Some(Err(bytes)) => {
                        return Some((Err(E::de(bytes)), (stream, buffer)));
                    }
                    None if buffer.is_empty() => return None,
                    None => {
                        let result = match std::str::from_utf8(&buffer) {
                            Ok(text) => Ok(text.to_owned()),
                            Err(error) => Err(utf8_error(error)),
                        };
                        buffer.clear();
                        return Some((result, (stream, buffer)));
                    }
                }
            }
        },
    )
}

impl<E, T, Request> IntoReq<StreamingText, Request, E> for T
where
    Request: ClientReq<E>,
    T: Into<TextStream<E>>,
    E: FromServerFnError,
{
    fn into_req(self, path: &str, accepts: &str) -> Result<Request, E> {
        let data = self.into();
        Request::try_new_post_streaming(
            path,
            accepts,
            StreamingText::CONTENT_TYPE,
            data.0.map(|chunk| chunk.unwrap_or_default().into()),
        )
    }
}

impl<E, T, Request> FromReq<StreamingText, Request, E> for T
where
    Request: Req<E> + Send + 'static,
    T: From<TextStream<E>> + 'static,
    E: FromServerFnError,
{
    async fn from_req(req: Request) -> Result<Self, E> {
        let data = req.try_into_stream()?;
        let s = TextStream::new(decode_text_stream(data));
        Ok(s.into())
    }
}

impl<E, Response> IntoRes<StreamingText, Response, E> for TextStream<E>
where
    Response: TryRes<E>,
    E: FromServerFnError,
{
    async fn into_res(self) -> Result<Response, E> {
        Response::try_from_stream(
            StreamingText::CONTENT_TYPE,
            self.into_inner()
                .map(|stream| stream.map(Into::into).map_err(|e| e.ser().body)),
        )
    }
}

impl<E, Response> FromRes<StreamingText, Response, E> for TextStream<E>
where
    Response: ClientRes<E> + Send,
    E: FromServerFnError,
{
    async fn from_res(res: Response) -> Result<Self, E> {
        let stream = res.try_into_stream()?;
        Ok(TextStream(Box::pin(decode_text_stream(stream))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{executor::block_on, stream};

    fn decode(
        chunks: Vec<Result<Bytes, Bytes>>,
    ) -> Vec<Result<String, ServerFnError>> {
        block_on(decode_text_stream(stream::iter(chunks)).collect())
    }

    fn assert_decodes(chunks: Vec<Result<Bytes, Bytes>>, expected: &str) {
        let decoded = decode(chunks);
        assert!(decoded.iter().all(Result::is_ok), "{decoded:?}");
        let text = decoded.into_iter().map(Result::unwrap).collect::<String>();
        assert_eq!(text, expected);
    }

    #[test]
    fn decodes_two_byte_character_split_across_chunks() {
        assert_decodes(
            vec![
                Ok(Bytes::from_static(&[b'a', 0xC3])),
                Ok(Bytes::from_static(&[0xA9, b'b'])),
            ],
            "aéb",
        );
    }

    #[test]
    fn decodes_four_byte_character_split_across_three_chunks() {
        assert_decodes(
            vec![
                Ok(Bytes::from_static(&[b'a', 0xF0])),
                Ok(Bytes::from_static(&[0x9F, 0x98])),
                Ok(Bytes::from_static(&[0x80, b'b'])),
            ],
            "a😀b",
        );
    }

    #[test]
    fn decodes_complete_utf8_chunk() {
        assert_decodes(vec![Ok(Bytes::from_static("aéb".as_bytes()))], "aéb");
    }

    #[test]
    fn drops_invalid_chunk_and_continues_decoding() {
        let decoded = decode(vec![
            Ok(Bytes::from_static(&[b'a', 0xFF, b'b'])),
            Ok(Bytes::from_static(b"c")),
        ]);

        assert!(matches!(
            &decoded[0],
            Err(ServerFnError::Deserialization(_))
        ));
        assert_eq!(&decoded[1], &Ok("c".to_string()));
    }

    #[test]
    fn reports_truncated_character_at_end_of_stream() {
        let decoded = decode(vec![Ok(Bytes::from_static(&[b'a', 0xC3]))]);

        assert_eq!(&decoded[0], &Ok("a".to_string()));
        assert!(matches!(
            &decoded[1],
            Err(ServerFnError::Deserialization(_))
        ));
        assert_eq!(decoded.len(), 2);
    }

    #[test]
    fn passes_through_stream_errors() {
        let error = ServerFnError::ServerError("transport error".to_string());
        let decoded = decode(vec![Err(error.ser().body)]);

        assert_eq!(decoded, vec![Err(error)]);
    }
}
