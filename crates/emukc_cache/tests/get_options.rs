//! `GetOption` configuration tests for Kache.

use emukc_cache::{Kache, KacheError};
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
async fn test_disable_local() {
    let (cache, temp) = setup_test_cache();

    let file_path = temp.path().join("test.png");
    std::fs::write(&file_path, b"test data").unwrap();

    // Disable local, should not find the file
    let opt = emukc_cache::GetOption::new().disable_local().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(matches!(result, Err(KacheError::FileNotFound(_))));
}

#[tokio::test]
async fn test_disable_mod() {
    let (cache, _temp) = setup_test_cache();

    let opt = emukc_cache::GetOption::new().disable_mod().disable_remote();
    let result = cache.get_with_opt("test.png", "", &opt).await;
    assert!(matches!(result, Err(KacheError::FileNotFound(_))));
}

#[tokio::test]
async fn test_api_mocking_option() {
    let opt = emukc_cache::GetOption::new_api_mocking();
    assert!(!opt.enable_local);
    assert!(!opt.enable_remote);
    assert!(opt.enable_mod);
    assert!(!opt.enable_shuffle);
}

#[tokio::test]
async fn test_remote_only_option() {
    let opt = emukc_cache::GetOption::new_remote_only();
    assert!(!opt.enable_local);
    assert!(opt.enable_remote);
    assert!(!opt.enable_mod);
    assert!(opt.enable_shuffle);
}

#[tokio::test]
async fn test_non_mod_option() {
    let opt = emukc_cache::GetOption::new_non_mod();
    assert!(opt.enable_local);
    assert!(opt.enable_remote);
    assert!(!opt.enable_mod);
    assert!(opt.enable_shuffle);
}
