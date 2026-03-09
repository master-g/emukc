//! In-memory LRU cache for version strings.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// In-memory cache for version strings to reduce database queries.
#[derive(Debug)]
pub struct VersionCache {
	cache: Mutex<LruCache<String, Option<String>>>,
}

impl VersionCache {
	/// Create a new version cache with the given capacity.
	pub fn new(capacity: usize) -> Self {
		Self {
			cache: Mutex::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
		}
	}

	/// Get a cached version string.
	pub fn get(&self, key: &str) -> Option<Option<String>> {
		self.cache.lock().unwrap().get(key).cloned()
	}

	/// Put a version string into the cache.
	pub fn put(&self, key: String, value: Option<String>) {
		self.cache.lock().unwrap().put(key, value);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cache_hit() {
		let cache = VersionCache::new(10);
		cache.put("key1".to_string(), Some("1.0.0".to_string()));
		assert_eq!(cache.get("key1"), Some(Some("1.0.0".to_string())));
	}

	#[test]
	fn test_cache_miss() {
		let cache = VersionCache::new(10);
		assert_eq!(cache.get("nonexistent"), None);
	}

	#[test]
	fn test_cache_none_value() {
		let cache = VersionCache::new(10);
		cache.put("key1".to_string(), None);
		assert_eq!(cache.get("key1"), Some(None));
	}

	#[test]
	fn test_lru_eviction() {
		let cache = VersionCache::new(2);
		cache.put("key1".to_string(), Some("1.0.0".to_string()));
		cache.put("key2".to_string(), Some("2.0.0".to_string()));
		cache.put("key3".to_string(), Some("3.0.0".to_string()));

		assert_eq!(cache.get("key1"), None); // Evicted
		assert_eq!(cache.get("key2"), Some(Some("2.0.0".to_string())));
		assert_eq!(cache.get("key3"), Some(Some("3.0.0".to_string())));
	}
}
