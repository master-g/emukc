//! Basic operations integration tests for Kache.

use emukc_cache::{Kache, KacheError};
use tempfile::TempDir;

fn setup_test_cache() -> (Kache, TempDir) {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://example.com".to_string())
		.with_gadgets_cdn("http://gadgets.example.com".to_string())
		.build()
		.unwrap();
	(cache, temp_dir)
}

#[tokio::test]
async fn test_builder_success() {
	let temp_dir = TempDir::new().unwrap();
	let result = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://cdn1.com".to_string())
		.with_gadgets_cdn("http://cdn2.com".to_string())
		.build();

	assert!(result.is_ok());
}

#[tokio::test]
async fn test_builder_missing_cache_root() {
	let result = Kache::builder()
		.with_content_cdn("http://cdn1.com".to_string())
		.with_gadgets_cdn("http://cdn2.com".to_string())
		.build();

	assert!(matches!(result, Err(KacheError::MissingField(_))));
}

#[tokio::test]
async fn test_file_not_found() {
	let (cache, _temp) = setup_test_cache();

	// Disable remote to ensure we get FileNotFound
	let opt = emukc_cache::GetOption::new().disable_remote();
	let result = cache.get_with_opt("nonexistent.png", "1.0.0", &opt).await;

	assert!(matches!(result, Err(KacheError::FileNotFound(_))));
}

#[tokio::test]
async fn test_local_file_exists() {
	let (cache, temp) = setup_test_cache();

	// Create a test file
	let file_path = temp.path().join("test.png");
	std::fs::write(&file_path, b"test data").unwrap();

	let result = cache.get("test.png", "").await;
	assert!(result.is_ok());
}
