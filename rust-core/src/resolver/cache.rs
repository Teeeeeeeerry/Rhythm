//! Resolution cache (ticket #187 split): storage, capacity/TTL pruning, and
//! the lookup/insert helpers. The orchestration (`resolve_url_impl`) lives
//! in the resolver module and calls into this.

use crate::resolver::ResolvedUrl;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Cached URL resolutions to avoid repeated yt-dlp calls.
/// Each entry stores the resolved info and the instant it was cached.
#[derive(Debug, Clone)]
pub(crate) struct CachedEntry {
    pub(crate) resolved: ResolvedUrl,
    pub(crate) cached_at: Instant,
}

pub(crate) static RESOLVED_CACHE: LazyLock<Mutex<HashMap<String, CachedEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Maximum number of entries in the resolution cache.
pub(crate) const CACHE_MAX_CAPACITY: usize = 256;

/// Time-to-live for a cached resolution result (configurable, #190;
/// default 1 hour — same as before).
static CACHE_TTL: LazyLock<Mutex<Duration>> =
    LazyLock::new(|| Mutex::new(Duration::from_secs(3600)));

/// Configure the resolution cache TTL. Defaults to 1 hour; callers of the
/// public resolve API are unaffected (ticket #190). Test seam for the
/// strategy tests; production keeps the default.
#[cfg(test)]
pub fn set_cache_ttl(ttl: Duration) {
    *CACHE_TTL.lock().unwrap() = ttl;
}

pub(crate) fn ttl() -> Duration {
    *CACHE_TTL.lock().unwrap()
}

/// Evict the oldest entry if the cache is over capacity, then remove any
/// entries whose TTL has expired.
pub(crate) fn prune_cache(cache: &mut HashMap<String, CachedEntry>) {
    // Capacity pruning: remove the oldest entry while over the limit.
    while cache.len() > CACHE_MAX_CAPACITY {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, e)| e.cached_at)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&oldest_key);
        } else {
            break;
        }
    }

    // TTL pruning.
    let now = Instant::now();
    cache.retain(|_, entry| now.duration_since(entry.cached_at) < ttl());
}

/// Cache lookup: fresh entry for `url`, or `None` (missing/expired).
pub(crate) fn get(url: &str) -> Option<ResolvedUrl> {
    let mut cache = RESOLVED_CACHE.lock().unwrap();
    prune_cache(&mut cache);
    match cache.get(url) {
        Some(entry) if entry.cached_at.elapsed() < ttl() => Some(entry.resolved.clone()),
        _ => None,
    }
}

/// Cache insert: store `resolved` for `url`.
pub(crate) fn put(url: &str, resolved: &ResolvedUrl) {
    let mut cache = RESOLVED_CACHE.lock().unwrap();
    prune_cache(&mut cache);
    cache.insert(
        url.to_string(),
        CachedEntry {
            resolved: resolved.clone(),
            cached_at: Instant::now(),
        },
    );
}

/// Remove a URL from the cache (the #120 recovery evicts the dead link
/// before a fresh re-resolution).
pub(crate) fn evict(url: &str) {
    RESOLVED_CACHE.lock().unwrap().remove(url);
}

