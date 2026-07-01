//! Filesystem support for serving static routes: atomic file writes and the
//! per-render header cache that pairs a cache-hit body with the headers
//! captured for the render that produced it.

use or_poisoned::OrPoisoned;

/// Identity of a written static file, used to pair a cache-hit response body
/// with the headers captured for the render that produced it.
///
/// An atomic rename preserves the inode and mtime, so a request that opens the
/// served file can recover the same identity the writer recorded and look up the
/// matching cached headers — with no lock spanning the file open and the header
/// read. On Unix the inode is authoritative; the length and mtime are also kept
/// so non-Unix targets still get a usable identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FileId {
    len: u64,
    mtime_ns: u128,
    #[cfg(unix)]
    ino: u64,
}

impl FileId {
    /// Derives a [`FileId`] from the metadata of an opened file. Read it from
    /// the same handle that serves the body, so the identity and the bytes can
    /// never come from different renders.
    pub fn from_metadata(md: &std::fs::Metadata) -> Self {
        let mtime_ns = md
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_nanos())
            .unwrap_or_default();
        Self {
            len: md.len(),
            mtime_ns,
            #[cfg(unix)]
            ino: std::os::unix::fs::MetadataExt::ino(md),
        }
    }
}

/// A static file written to a temp location and ready to be published.
///
/// Created by [`stage_file_atomic`]. The bytes are already on disk under a
/// unique temp name, and [`id`](Self::id) is known, but the file is not yet
/// visible at its target path until [`commit`](Self::commit) renames it there.
/// This lets a caller record the [`FileId`]'s headers *before* the file becomes
/// servable, so a concurrent reader that opens it always finds matching headers.
///
/// Dropping a `StagedFile` without committing removes the temp file on a
/// best-effort basis, so abandoned writes don't accumulate in the site root.
#[must_use = "a staged file is not visible until `.commit()` is called"]
pub struct StagedFile {
    tmp_path: std::path::PathBuf,
    target: std::path::PathBuf,
    id: FileId,
    committed: bool,
}

impl StagedFile {
    /// The identity the file will have once committed. The rename preserves it,
    /// so a reader that opens the committed file recovers the same value.
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Atomically renames the staged file over its target path. On failure the
    /// temp file is removed on a best-effort basis.
    pub async fn commit(mut self) -> std::io::Result<()> {
        self.committed = true;
        if let Err(err) = tokio::fs::rename(&self.tmp_path, &self.target).await
        {
            let _ = tokio::fs::remove_file(&self.tmp_path).await;
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if !self.committed {
            // Best-effort, synchronous: this only runs on an abandoned write.
            let _ = std::fs::remove_file(&self.tmp_path);
        }
    }
}

/// Writes `contents` to a uniquely-named temp file in `path`'s directory,
/// ready to be atomically published with [`StagedFile::commit`].
///
/// Writing to a temp file and renaming it into place (rather than writing in
/// place, e.g. `tokio::fs::write`, which truncates the target up front) means a
/// concurrent reader or a crash never observes a half-written or truncated file.
/// Missing parent directories are created first.
pub async fn stage_file_atomic(
    path: &std::path::Path,
    contents: &[u8],
) -> std::io::Result<StagedFile> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let tmp_path = {
        static COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut file_name = path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        file_name.push(format!(".tmp.{}.{n}", std::process::id()));
        path.with_file_name(file_name)
    };

    if let Err(err) = tokio::fs::write(&tmp_path, contents).await {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(err);
    }
    // Capture the identity from the temp file: the rename in `commit` preserves
    // the inode and mtime, so this is what a reader sees once the file is in
    // place.
    let id = match tokio::fs::metadata(&tmp_path).await {
        Ok(md) => FileId::from_metadata(&md),
        Err(err) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(err);
        }
    };

    Ok(StagedFile {
        tmp_path,
        target: path.to_path_buf(),
        id,
        committed: false,
    })
}

/// Default upper bound on the number of per-path header snapshots cached for
/// static routes. Without a bound the cache grew for the life of the process,
/// one entry per unique static path served (e.g. attacker-driven slugs on a
/// regenerated `/posts/{slug}` route).
///
/// Eviction is graceful: the static file is still served from disk, the cache
/// only drops the custom headers/status captured at generation time for the
/// evicted path (re-populated on the next regeneration). 1024 covers a typical
/// static site's working set for a worst case on the order of ~1 MB.
pub const STATIC_HEADERS_DEFAULT_CAPACITY: std::num::NonZeroUsize =
    match std::num::NonZeroUsize::new(1024) {
        Some(capacity) => capacity,
        None => unreachable!(),
    };

/// Environment variable that overrides [`STATIC_HEADERS_DEFAULT_CAPACITY`].
/// A missing, unparseable, or zero value falls back to the default.
pub const STATIC_HEADERS_CAPACITY_ENV: &str =
    "LEPTOS_STATIC_HEADERS_CACHE_SIZE";

/// Default number of recent renders' headers to keep per path. A cache hit
/// matches the file it opened against these, so this only needs to cover the
/// renders whose files might still be open: the one currently on disk plus the
/// one it just replaced. An older handle (more than this many regenerations
/// behind) falls back to default headers, the same graceful degradation as
/// eviction.
pub const CACHED_GENERATIONS_DEFAULT: std::num::NonZeroUsize =
    match std::num::NonZeroUsize::new(2) {
        Some(generations) => generations,
        None => unreachable!(),
    };

/// Environment variable that overrides [`CACHED_GENERATIONS_DEFAULT`].
/// A missing, unparseable, or zero value falls back to the default.
pub const CACHED_GENERATIONS_ENV: &str = "LEPTOS_STATIC_HEADERS_GENERATIONS";

/// A bounded, per-path cache of the response headers/status captured when a
/// static route was rendered, keyed by the [`FileId`] of the file each snapshot
/// was written with.
///
/// A cache hit takes the identity from the file handle it is about to serve and
/// looks the headers up by that identity, so the body and the headers always
/// come from the same render even while the path is being regenerated
/// concurrently — with no lock spanning the file open and the header lookup. The
/// last [`CACHED_GENERATIONS_DEFAULT`] renders (overridable via
/// [`CACHED_GENERATIONS_ENV`]) are kept per path — the file currently on disk
/// plus the one it just replaced; an older handle falls back to default headers,
/// the same graceful degradation as eviction.
///
/// Generic over the integration's response-parts type `P`, which differs per
/// web framework.
pub struct StaticHeadersCache<P> {
    inner: std::sync::RwLock<lru::LruCache<String, Vec<(FileId, P)>>>,
    max_generations: usize,
}

impl<P: Clone> StaticHeadersCache<P> {
    /// Creates a cache configured from the environment: capacity from
    /// [`STATIC_HEADERS_CAPACITY_ENV`] and per-path generations from
    /// [`CACHED_GENERATIONS_ENV`], each falling back to its default
    /// ([`STATIC_HEADERS_DEFAULT_CAPACITY`] / [`CACHED_GENERATIONS_DEFAULT`])
    /// when missing, unparseable, or zero.
    pub fn from_env() -> Self {
        let capacity = env_non_zero(STATIC_HEADERS_CAPACITY_ENV)
            .unwrap_or(STATIC_HEADERS_DEFAULT_CAPACITY);
        let max_generations = env_non_zero(CACHED_GENERATIONS_ENV)
            .unwrap_or(CACHED_GENERATIONS_DEFAULT)
            .get();
        Self {
            inner: std::sync::RwLock::new(lru::LruCache::new(capacity)),
            max_generations,
        }
    }

    /// Records the headers captured for a freshly written static file, keyed by
    /// the file's identity, keeping the most recent generations.
    ///
    /// Call this *before* the file is made visible at its target path, so a
    /// concurrent cache hit that opens it always finds the matching headers.
    pub fn record(&self, path: &str, id: FileId, parts: P) {
        let mut cache = self.inner.write().or_poisoned();
        if let Some(generations) = cache.get_mut(path) {
            generations.push((id, parts));
            while generations.len() > self.max_generations {
                generations.remove(0);
            }
        } else {
            cache.put(path.to_string(), vec![(id, parts)]);
        }
    }

    /// Looks up the headers cached for the exact file identified by `id`. `None`
    /// means the matching render was evicted or is older than the retained
    /// generations; the body is then served with default headers, the same
    /// graceful degradation as a cache miss.
    pub fn get(&self, path: &str, id: FileId) -> Option<P> {
        self.inner
            .write()
            .or_poisoned()
            .get(path)
            .and_then(|generations| {
                generations
                    .iter()
                    .rev()
                    .find(|(cached_id, _)| *cached_id == id)
                    .map(|(_, parts)| parts.clone())
            })
    }
}

/// Reads a positive `usize` from environment variable `name`, returning `None`
/// when it is unset, unparseable, or zero.
fn env_non_zero(name: &str) -> Option<std::num::NonZeroUsize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(std::num::NonZeroUsize::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Staging then committing must create the file (and any missing parents)
    // with its full contents and leave no temp file behind, so a crash mid-write
    // can never expose a truncated or empty file to a reader.
    #[tokio::test]
    async fn stage_file_atomic_writes_full_contents_without_leftovers() {
        let dir = std::env::temp_dir().join(format!(
            "leptos_integration_utils_atomic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        // a missing parent directory is created on the way to the target
        let target = dir.join("nested").join("page.html");
        let contents = b"<html><body>hello</body></html>";
        stage_file_atomic(&target, contents)
            .await
            .unwrap()
            .commit()
            .await
            .unwrap();

        // the target was written in full
        assert_eq!(std::fs::read(&target).unwrap(), contents);

        // no temp file was left behind alongside it
        let leftovers = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(leftovers, 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    // A cache hit recovers exactly the headers captured for the render whose
    // file it opened, so a body and its headers can never come from different
    // renders. An identity with no recorded render degrades to a miss.
    #[test]
    fn static_headers_cache_pairs_by_file_id() {
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::from_env();
        let old = FileId {
            len: 1,
            ..Default::default()
        };
        let new = FileId {
            len: 2,
            ..Default::default()
        };

        cache.record("/post", old, 1);
        cache.record("/post", new, 2);

        assert_eq!(cache.get("/post", old), Some(1));
        assert_eq!(cache.get("/post", new), Some(2));
        // an unknown identity (evicted or too old) and an unknown path both miss
        assert_eq!(
            cache.get(
                "/post",
                FileId {
                    len: 3,
                    ..Default::default()
                }
            ),
            None
        );
        assert_eq!(cache.get("/missing", new), None);
    }

    // Only the most recent generations are retained per path; an older handle
    // falls back to default headers, the same graceful degradation as eviction.
    #[test]
    fn static_headers_cache_keeps_recent_generations() {
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::from_env();
        let kept = CACHED_GENERATIONS_DEFAULT.get();
        let ids: Vec<FileId> = (0..(kept as u64 + 2))
            .map(|len| FileId {
                len,
                ..Default::default()
            })
            .collect();
        for (generation, id) in ids.iter().enumerate() {
            cache.record("/post", *id, generation as u32);
        }

        // the most recent `kept` renders survive ...
        for offset in 1..=kept {
            let generation = ids.len() - offset;
            assert_eq!(
                cache.get("/post", ids[generation]),
                Some(generation as u32)
            );
        }
        // ... and the render just before them has been dropped
        assert_eq!(cache.get("/post", ids[ids.len() - kept - 1]), None);
    }

    // The per-path cache must never grow without bound: serving many unique
    // static paths (e.g. attacker-driven slugs) used to leak one entry per path
    // for the life of the process.
    #[test]
    fn static_headers_cache_is_bounded() {
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::from_env();
        let capacity = STATIC_HEADERS_DEFAULT_CAPACITY.get();
        let id = FileId::default();
        for i in 0..(capacity + 10) {
            cache.record(&format!("/post/{i}"), id, i as u32);
        }

        // the earliest-inserted paths have been evicted ...
        assert_eq!(cache.get("/post/0", id), None);
        // ... while a recently-inserted one is still present
        assert_eq!(
            cache.get(&format!("/post/{}", capacity + 9), id),
            Some((capacity + 9) as u32)
        );
    }
}
