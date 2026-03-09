//! CDN selection and URL building tests for Kache.

use emukc_cache::Kache;
use tempfile::TempDir;

#[allow(dead_code)]
fn setup_test_cache() -> (Kache, TempDir) {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://content1.com".to_string())
		.with_content_cdn("http://content2.com".to_string())
		.with_gadgets_cdn("http://gadgets1.com".to_string())
		.with_gadgets_cdn("http://gadgets2.com".to_string())
		.build()
		.unwrap();
	(cache, temp_dir)
}

#[tokio::test]
async fn test_multiple_content_cdns() {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdns(vec![
			"http://cdn1.com".to_string(),
			"http://cdn2.com".to_string(),
		])
		.with_gadgets_cdn("http://gadgets.com".to_string())
		.build();

	assert!(cache.is_ok());
}

#[tokio::test]
async fn test_multiple_gadgets_cdns() {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://content.com".to_string())
		.with_gadgets_cdns(vec![
			"http://gadgets1.com".to_string(),
			"http://gadgets2.com".to_string(),
		])
		.build();

	assert!(cache.is_ok());
}

#[tokio::test]
async fn test_builder_with_proxy() {
	let temp_dir = TempDir::new().unwrap();
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://cdn.com".to_string())
		.with_gadgets_cdn("http://gadgets.com".to_string())
		.with_proxy(Some("http://proxy.com:8080".to_string()))
		.build();

	assert!(cache.is_ok());
}

#[tokio::test]
async fn test_builder_with_custom_db_path() {
	let temp_dir = TempDir::new().unwrap();
	let db_path = temp_dir.path().join("custom.db");
	let cache = Kache::builder()
		.with_cache_root(temp_dir.path().to_path_buf())
		.with_content_cdn("http://cdn.com".to_string())
		.with_gadgets_cdn("http://gadgets.com".to_string())
		.with_db_path(db_path.to_str().unwrap().to_string())
		.build();

	assert!(cache.is_ok());
	assert!(db_path.exists());
}
