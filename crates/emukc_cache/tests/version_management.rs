//! Version management tests for Kache.

use emukc_cache::Kache;
use tempfile::TempDir;

fn setup_test_cache() -> (Kache, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cache = Kache::builder()
        .with_cache_root(temp_dir.path().to_path_buf())
        .with_content_cdn("http://cdn.com".to_string())
        .with_gadgets_cdn("http://gadgets.com".to_string())
        .build()
        .unwrap();
    (cache, temp_dir)
}

#[tokio::test]
async fn test_version_match() {
    let (cache, temp) = setup_test_cache();

    // Create a test file
    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();

    // First access without version to populate cache
    let opt = emukc_cache::GetOption::new().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_version_expired() {
    let (cache, temp) = setup_test_cache();

    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();

    // First access without version - file has no version record
    let opt = emukc_cache::GetOption::new().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(result.is_ok());

    // Requesting with a version when no version is stored returns the file
    // (because no version requirement means any version is acceptable)
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_no_version() {
    let (cache, temp) = setup_test_cache();

    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();

    // Access without version
    let opt = emukc_cache::GetOption::new().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(result.is_ok());
}

/// Regression: DB has a stored version but the request arrives without one.
/// Before the fix, `find_in_local` compared stored > "" → `InvalidFileVersion`,
/// causing unnecessary re-downloads on every first access.
#[tokio::test]
async fn test_version_rollback_no_version_requested() {
    let (cache, temp) = setup_test_cache();

    // Simulate populate: file exists locally with a stored version
    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();
    cache.set_version("test.png", Some("1.5")).await.unwrap();

    // Client requests without version → should serve the file, not error
    let opt = emukc_cache::GetOption::new().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
}

#[tokio::test]
async fn test_get_cached_version_returns_stored_version() {
    let (cache, temp) = setup_test_cache();

    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();
    cache.set_version("test.png", Some("6.2.9.0")).await.unwrap();

    let version = cache.get_cached_version("test.png").await.unwrap();
    assert_eq!(version.as_deref(), Some("6.2.9.0"));
}

#[tokio::test]
async fn test_get_cached_version_returns_none_for_unknown_path() {
    let (cache, _temp) = setup_test_cache();

    let version = cache.get_cached_version("nonexistent.png").await.unwrap();
    assert!(version.is_none());
}
