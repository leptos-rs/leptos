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
/// read. On Unix the inode makes the identity exact. Without an inode, identity
/// is (length, mtime): equal-length renders written within one filesystem
/// timestamp tick are indistinguishable. A request that opened the replaced file
/// just before such a re-render may be paired with the newer render's headers.
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
/// visible at its target path until [`StaticHeadersCache::publish`] renames it.
/// This records the [`FileId`]'s headers *before* the file becomes
/// servable, so a concurrent reader that opens it always finds matching headers.
///
/// Dropping a `StagedFile` without committing removes the temp file on a
/// best-effort basis, so abandoned writes don't accumulate in the site root.
#[must_use = "a staged file is not visible until `StaticHeadersCache::publish` \
              is called"]
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
    async fn commit(mut self) -> std::io::Result<()> {
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
/// ready to be atomically published with [`StaticHeadersCache::publish`].
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
/// static site's working set: ~2 MB at ~1 KB per snapshot with up to
/// [`CACHED_GENERATIONS_DEFAULT`] snapshots per path.
pub const STATIC_HEADERS_DEFAULT_CAPACITY: std::num::NonZeroUsize =
    match std::num::NonZeroUsize::new(1024) {
        Some(capacity) => capacity,
        None => unreachable!(),
    };

/// Environment variable that overrides [`STATIC_HEADERS_DEFAULT_CAPACITY`].
/// A missing, unparseable, or zero value falls back to the default.
pub const STATIC_HEADERS_CAPACITY_ENV: &str =
    "LEPTOS_STATIC_HEADERS_CACHE_SIZE";

/// Default number of recent renders' headers to keep per path. Serialized
/// publishes keep the newest render on disk; the previous render covers requests
/// that opened it just before replacement. Older handles re-open the file via
/// [`StaticHeadersCache::pair`], so two retained renders suffice.
pub const CACHED_GENERATIONS_DEFAULT: std::num::NonZeroUsize =
    match std::num::NonZeroUsize::new(2) {
        Some(generations) => generations,
        None => unreachable!(),
    };

// One re-open finds recorded headers; a second covers racing regeneration.
const REOPEN_ATTEMPTS: usize = 2;

/// A bounded, per-path cache of the response headers/status captured when a
/// static route was rendered, keyed by the [`FileId`] of the file each snapshot
/// was written with.
///
/// [`publish`](Self::publish) records headers before renaming the file into
/// place. [`pair`](Self::pair) matches an opened file's identity to its headers,
/// re-opening handles older than the retained renders. By default the last
/// [`CACHED_GENERATIONS_DEFAULT`] renders are kept per path.
///
/// Generic over the integration's response-parts type `P`, which differs per
/// web framework.
pub struct StaticHeadersCache<P> {
    inner: std::sync::RwLock<lru::LruCache<String, Vec<(FileId, P)>>>,
    max_generations: usize,
    /// Serializes records and renames in the same order, keeping the file on disk
    /// among retained renders. Held only for recording and renaming, never for
    /// writing the file; readers never take it.
    publish_lock: tokio::sync::Mutex<()>,
}

impl<P: Clone> StaticHeadersCache<P> {
    /// Creates a cache with explicit path capacity and per-path generations.
    pub fn new(
        capacity: std::num::NonZeroUsize,
        generations: std::num::NonZeroUsize,
    ) -> Self {
        Self {
            inner: std::sync::RwLock::new(lru::LruCache::new(capacity)),
            max_generations: generations.get(),
            publish_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Creates a cache with capacity from [`STATIC_HEADERS_CAPACITY_ENV`],
    /// falling back to [`STATIC_HEADERS_DEFAULT_CAPACITY`] when missing,
    /// unparseable, or zero, and [`CACHED_GENERATIONS_DEFAULT`] renders per path.
    pub fn from_env() -> Self {
        Self::new(
            env_non_zero(STATIC_HEADERS_CAPACITY_ENV)
                .unwrap_or(STATIC_HEADERS_DEFAULT_CAPACITY),
            CACHED_GENERATIONS_DEFAULT,
        )
    }

    /// Records headers before atomically publishing the staged file.
    pub async fn publish(
        &self,
        path: &str,
        staged: StagedFile,
        parts: Option<P>,
    ) -> std::io::Result<()> {
        let _publish = self.publish_lock.lock().await;
        if let Some(parts) = parts {
            self.record(path, staged.id(), parts);
        }
        staged.commit().await
    }

    /// Pairs a handle with its render's headers, re-opening at most twice if
    /// older than retained renders. Unknown paths return immediately without
    /// headers; re-open errors propagate.
    pub async fn pair<H, E, Fut>(
        &self,
        path: &str,
        mut handle: H,
        id: impl Fn(&H) -> FileId,
        mut reopen: impl FnMut() -> Fut,
    ) -> Result<(H, Option<P>), E>
    where
        Fut: std::future::Future<Output = Result<H, E>>,
    {
        for _ in 0..REOPEN_ATTEMPTS {
            if let Some(parts) = self.get(path, id(&handle)) {
                return Ok((handle, Some(parts)));
            }
            // Never-rendered or evicted paths have no headers to find by
            // re-opening the file.
            if !self.inner.read().or_poisoned().contains(path) {
                return Ok((handle, None));
            }
            handle = reopen().await?;
        }
        let parts = self.get(path, id(&handle));
        Ok((handle, parts))
    }

    /// Records the headers captured for a freshly written static file, keyed by
    /// the file's identity, keeping the most recent generations.
    ///
    /// Call this *before* the file is made visible at its target path, so a
    /// concurrent cache hit that opens it always finds the matching headers.
    fn record(&self, path: &str, id: FileId, parts: P) {
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
    /// generations; see [`pair`](Self::pair).
    fn get(&self, path: &str, id: FileId) -> Option<P> {
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
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::new(
            STATIC_HEADERS_DEFAULT_CAPACITY,
            CACHED_GENERATIONS_DEFAULT,
        );
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

    // Only the most recent generations are retained for direct lookup.
    #[test]
    fn static_headers_cache_keeps_recent_generations() {
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::new(
            STATIC_HEADERS_DEFAULT_CAPACITY,
            CACHED_GENERATIONS_DEFAULT,
        );
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
        let cache: StaticHeadersCache<u32> = StaticHeadersCache::new(
            STATIC_HEADERS_DEFAULT_CAPACITY,
            CACHED_GENERATIONS_DEFAULT,
        );
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

    // Concurrent renders of one path must leave the file on disk paired with
    // its headers, which requires serialized publishes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn concurrent_publishes_retain_headers_for_file_on_disk() {
        use std::io::Read;

        let dir = std::env::temp_dir().join(format!(
            "leptos_integration_utils_publish_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = dir.join("page.html");
        let cache = std::sync::Arc::new(StaticHeadersCache::new(
            STATIC_HEADERS_DEFAULT_CAPACITY,
            CACHED_GENERATIONS_DEFAULT,
        ));
        for _ in 0..50 {
            let mut tasks = Vec::new();
            for i in 0..16 {
                let cache = cache.clone();
                let target = target.clone();
                tasks.push(tokio::spawn(async move {
                    let staged = stage_file_atomic(
                        &target,
                        format!("body-{i}").as_bytes(),
                    )
                    .await
                    .unwrap();
                    cache.publish("/p", staged, Some(i)).await.unwrap();
                }));
            }
            for task in tasks {
                task.await.unwrap();
            }
            let mut file = std::fs::File::open(&target).unwrap();
            let id = FileId::from_metadata(&file.metadata().unwrap());
            let mut body = String::new();
            file.read_to_string(&mut body).unwrap();
            let i: usize = body.strip_prefix("body-").unwrap().parse().unwrap();
            assert_eq!(cache.get("/p", id), Some(i));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    // Older handles need bounded re-opens to recover retained headers; paths
    // with no retained renders cannot benefit from re-opening.
    #[tokio::test]
    async fn pair_reopens_only_stale_handles_with_a_fixed_bound() {
        let cache = StaticHeadersCache::new(
            STATIC_HEADERS_DEFAULT_CAPACITY,
            CACHED_GENERATIONS_DEFAULT,
        );
        let stale = FileId::default();
        let current = FileId { len: 1, ..stale };
        cache.record("/p", current, 7);
        for (path, handle, reopened, expected, calls) in [
            ("/p", current, current, Some(7), 0),
            ("/p", stale, current, Some(7), 1),
            ("/unknown", stale, current, None, 0),
            ("/p", stale, stale, None, REOPEN_ATTEMPTS),
        ] {
            let mut reopens = 0;
            let result = cache
                .pair(
                    path,
                    handle,
                    |h: &FileId| *h,
                    || {
                        reopens += 1;
                        std::future::ready(Ok::<_, ()>(reopened))
                    },
                )
                .await
                .unwrap();
            assert_eq!(
                result,
                (if calls == 0 { handle } else { reopened }, expected)
            );
            assert_eq!(reopens, calls);
        }
    }
}
