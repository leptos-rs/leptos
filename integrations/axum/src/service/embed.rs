use futures::Future;
use rust_embed::{EmbeddedFile, RustEmbed};
use std::{
    borrow::Cow,
    io::{self, Cursor, ErrorKind, SeekFrom},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use tower_http::services::fs::{Backend, File, Metadata};

/// This is converted from a [`EmbeddedFile`] as part of the conversion to an [`EmbeddedEntry`].
#[derive(Clone)]
pub struct EmbeddedMetadata {
    modified: SystemTime,
    len: u64,
}

impl Metadata for EmbeddedMetadata {
    fn is_dir(&self) -> bool {
        // As they are all files, they won't be a dir.
        false
    }

    fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }

    fn len(&self) -> u64 {
        self.len
    }
}

impl From<EmbeddedFile> for EmbeddedMetadata {
    fn from(EmbeddedFile { data, metadata }: EmbeddedFile) -> Self {
        Self {
            len: data.len() as u64,
            modified: UNIX_EPOCH
                + Duration::from_secs(metadata.last_modified().unwrap_or(0)),
        }
    }
}

/// This is created from an [`EmbeddedFile`] so that it may be used with our implementation of a
/// ServeDir backend.
#[derive(Clone)]
pub struct EmbeddedEntry {
    cursor: Cursor<Cow<'static, [u8]>>,
    metadata: EmbeddedMetadata,
}

impl From<EmbeddedFile> for EmbeddedEntry {
    fn from(EmbeddedFile { data, metadata }: EmbeddedFile) -> Self {
        Self {
            // Assume no usize exceed the limits of a `u64`.  Caching this value to avoid needing the
            // currently unstable `Seek::stream_len()`.
            metadata: EmbeddedMetadata {
                len: data.len() as u64,
                modified: UNIX_EPOCH
                    + Duration::from_secs(
                        metadata.last_modified().unwrap_or(0),
                    ),
            },
            cursor: Cursor::new(data),
        }
    }
}

impl AsyncRead for EmbeddedEntry {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.cursor), cx, buf)
    }
}

impl AsyncSeek for EmbeddedEntry {
    fn start_seek(
        mut self: Pin<&mut Self>,
        position: SeekFrom,
    ) -> io::Result<()> {
        AsyncSeek::start_seek(Pin::new(&mut self.cursor), position)
    }

    fn poll_complete(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<u64>> {
        AsyncSeek::poll_complete(Pin::new(&mut self.cursor), cx)
    }
}

impl File for EmbeddedEntry {
    type Metadata = EmbeddedMetadata;
    type MetadataFuture<'a> =
        Pin<Box<dyn Future<Output = io::Result<Self::Metadata>> + Send + 'a>>;

    fn metadata(&self) -> Self::MetadataFuture<'_> {
        Box::pin(async { Ok(self.metadata.clone()) })
    }
}

/// This implements a [`Backend`] for use with [`ServeDir`] that serves files using [`RustEmbed`].
#[derive(Clone)]
pub struct EmbeddedSiteRoot<SR> {
    _site_root: SR,
}

impl<SR> EmbeddedSiteRoot<SR> {
    /// Create a new embedded site root backend for use with [`ServeDir`].
    pub fn new(site_root: SR) -> Self {
        Self {
            _site_root: site_root,
        }
    }
}

impl<SR> Backend for EmbeddedSiteRoot<SR>
where
    SR: RustEmbed + Clone + Send + Sync + 'static,
{
    type File = EmbeddedEntry;
    type Metadata = EmbeddedMetadata;
    type OpenFuture =
        Pin<Box<dyn Future<Output = io::Result<EmbeddedEntry>> + Send>>;
    type MetadataFuture =
        Pin<Box<dyn Future<Output = io::Result<EmbeddedMetadata>> + Send>>;

    fn open(&self, path: PathBuf) -> Self::OpenFuture {
        Box::pin(async move {
            path.as_os_str()
                .to_str()
                .and_then(SR::get)
                .map(EmbeddedEntry::from)
                .ok_or_else(|| ErrorKind::NotFound.into())
        })
    }

    fn metadata(&self, path: PathBuf) -> Self::MetadataFuture {
        Box::pin(async move {
            path.as_os_str()
                .to_str()
                .and_then(SR::get)
                .map(EmbeddedMetadata::from)
                .ok_or_else(|| ErrorKind::NotFound.into())
        })
    }
}
