use futures::future::{Either, Ready, ready};
use std::{
    io::{self, ErrorKind},
    path::{Component, Path, PathBuf},
};
use tower_http::services::fs::Backend;

/// A [`Backend`] for [`ServeDir`] that serves the files of another backend,
/// except the files in one directory.
///
/// The site root services use it to skip the
/// [`hidden_site_pkg_dir`](leptos::config::LeptosOptions::hidden_site_pkg_dir),
/// so the site pkg has one URL.
///
/// [`ServeDir`]: tower_http::services::ServeDir
#[derive(Clone, Debug)]
pub struct ExcludeDir<B> {
    backend: B,
    dir: Option<PathBuf>,
}

impl<B> ExcludeDir<B> {
    /// Serves the files of `backend`, except the files in `dir`.
    ///
    /// `ServeDir` gives the backend its own path joined with the request path,
    /// so `dir` must start with that path too.
    pub fn new(backend: B, dir: Option<PathBuf>) -> Self {
        Self { backend, dir }
    }

    fn excludes(&self, path: &Path) -> bool {
        // `ServeDir` starts a relative path with `.`, so skip `.` parts.
        fn parts(path: &Path) -> Vec<Component<'_>> {
            path.components()
                .filter(|part| *part != Component::CurDir)
                .collect()
        }
        self.dir
            .as_ref()
            .is_some_and(|dir| parts(path).starts_with(&parts(dir)))
    }
}

impl<B: Backend> Backend for ExcludeDir<B> {
    type File = B::File;
    type Metadata = B::Metadata;
    type OpenFuture = Either<B::OpenFuture, Ready<io::Result<B::File>>>;
    type MetadataFuture =
        Either<B::MetadataFuture, Ready<io::Result<B::Metadata>>>;

    fn open(&self, path: PathBuf) -> Self::OpenFuture {
        if self.excludes(&path) {
            Either::Right(ready(Err(ErrorKind::NotFound.into())))
        } else {
            Either::Left(self.backend.open(path))
        }
    }

    fn metadata(&self, path: PathBuf) -> Self::MetadataFuture {
        if self.excludes(&path) {
            Either::Right(ready(Err(ErrorKind::NotFound.into())))
        } else {
            Either::Left(self.backend.metadata(path))
        }
    }
}
