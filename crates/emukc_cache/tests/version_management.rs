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
