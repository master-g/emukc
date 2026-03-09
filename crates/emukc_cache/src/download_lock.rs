//! Download lock to prevent concurrent downloads of the same file.

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Prevents concurrent downloads of the same file.
#[derive(Debug)]
pub struct DownloadLock {
	locks: DashMap<String, Arc<Semaphore>>,
}

impl DownloadLock {
	/// Create a new download lock.
	pub fn new() -> Self {
		Self {
			locks: DashMap::new(),
		}
	}

	/// Acquire a lock for the given key.
	pub async fn acquire(&self, key: &str) -> OwnedSemaphorePermit {
		let sem =
			self.locks.entry(key.to_owned()).or_insert_with(|| Arc::new(Semaphore::new(1))).clone();

		sem.acquire_owned().await.unwrap()
	}
}

impl Default for DownloadLock {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_basic_lock() {
		let lock = DownloadLock::new();
		let _permit = lock.acquire("file1").await;
		// Lock acquired successfully
	}

	#[tokio::test]
	async fn test_concurrent_same_file() {
		let lock = Arc::new(DownloadLock::new());
		let lock1 = lock.clone();
		let lock2 = lock.clone();

		let handle1 = tokio::spawn(async move {
			let _permit = lock1.acquire("file1").await;
			tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		});

		let handle2 = tokio::spawn(async move {
			tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
			let _permit = lock2.acquire("file1").await;
		});

		let start = std::time::Instant::now();
		let _ = tokio::join!(handle1, handle2);
		let elapsed = start.elapsed();

		// Second task should wait for first
		assert!(elapsed.as_millis() >= 100);
	}

	#[tokio::test]
	async fn test_concurrent_different_files() {
		let lock = Arc::new(DownloadLock::new());
		let lock1 = lock.clone();
		let lock2 = lock.clone();

		let handle1 = tokio::spawn(async move {
			let _permit = lock1.acquire("file1").await;
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		});

		let handle2 = tokio::spawn(async move {
			let _permit = lock2.acquire("file2").await;
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		});

		let start = std::time::Instant::now();
		let _ = tokio::join!(handle1, handle2);
		let elapsed = start.elapsed();

		// Both should run concurrently
		assert!(elapsed.as_millis() < 100);
	}
}
