//! TTL-based response cache wrapping `moka::future::Cache`.

use std::time::Duration;

use moka::future::Cache;

/// TTL-based response cache shared across all tools in a server.
#[derive(Clone)]
pub struct ResponseCache {
    inner: Cache<String, serde_json::Value>,
}

impl ResponseCache {
    /// Create a new cache with the given maximum capacity and default TTL.
    #[must_use]
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    /// Get a cached value by key, or fetch and cache it using the provided
    /// async closure on cache miss.
    ///
    /// # Errors
    ///
    /// Returns any error produced by `fetch` on a cache miss.
    pub async fn get_or_fetch<F, Fut>(
        &self,
        key: &str,
        fetch: F,
    ) -> Result<serde_json::Value, crate::McpServerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<serde_json::Value, crate::McpServerError>>,
    {
        if let Some(cached) = self.inner.get(key).await {
            tracing::debug!(key, "cache hit");
            return Ok(cached);
        }

        tracing::debug!(key, "cache miss, fetching");
        let value = fetch().await?;
        self.inner.insert(key.to_owned(), value.clone()).await;
        Ok(value)
    }

    /// Insert a value directly into the cache.
    pub async fn insert(&self, key: String, value: serde_json::Value) {
        self.inner.insert(key, value).await;
    }

    /// Remove a cached entry.
    pub async fn invalidate(&self, key: &str) {
        self.inner.invalidate(key).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_hit_returns_stored_value() {
        let cache = ResponseCache::new(100, Duration::from_secs(60));
        let value = serde_json::json!({"status": "ok"});
        cache.insert("test_key".to_owned(), value.clone()).await;

        let result = cache
            .get_or_fetch("test_key", || async {
                panic!("should not fetch on cache hit");
            })
            .await
            .unwrap();

        assert_eq!(result, value);
    }

    #[tokio::test]
    async fn cache_miss_calls_fetch() {
        let cache = ResponseCache::new(100, Duration::from_secs(60));
        let expected = serde_json::json!({"fetched": true});
        let expected_clone = expected.clone();

        let result = cache
            .get_or_fetch("missing_key", || async { Ok(expected_clone) })
            .await
            .unwrap();

        assert_eq!(result, expected);
    }
}
