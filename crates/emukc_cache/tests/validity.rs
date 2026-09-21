//! Cache validity tests for `Kache`.
//!
//! `is_valid` is private, so each case places a file in the cache root and calls
//! `get` with remote disabled: `Ok` means the file was accepted, `Err` means it
//! was rejected.
//!
//! Note which `Err` comes back. `get_with_opt` logs the internal `InvalidFile`
//! and falls through to the remote branch; with remote disabled the caller ends
//! up with `FileNotFound`. So a corrupt cache entry and an absent one are
//! indistinguishable from outside. Plan 007 split the *CDN-side* outcomes (a 404
//! versus no CDN answering at all); this local-side collapse is untouched by it.
//!
//! Baseline for plan 008 — two of these pin behaviour that is known to be wrong
//! and that 008 is expected to flip. Those carry a comment saying so.

use emukc_cache::{GetOption, Kache, KacheError};
use tempfile::TempDir;

const HTML_ERROR_PAGE: &[u8] =
    b"<!DOCTYPE html>\n<html><head><title>404</title></head><body>nope</body></html>";

fn local_only_cache(root: &TempDir) -> Kache {
    Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdn("http://content.invalid".to_string())
        .with_gadgets_cdn("http://gadgets.invalid".to_string())
        .build()
        .unwrap()
}

/// Place `body` at `rel` inside the cache root, then ask for it without remote.
async fn get_local(rel: &str, body: &[u8]) -> (TempDir, Result<tokio::fs::File, KacheError>) {
    let root = TempDir::new().unwrap();
    let full = root.path().join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, body).unwrap();

    let cache = local_only_cache(&root);
    let opt = GetOption::new().disable_remote();
    let result = cache.get_with_opt(rel, "", &opt).await;
    (root, result)
}

#[tokio::test]
async fn ordinary_non_empty_file_is_valid() {
    let (_root, result) = get_local("kcs2/resources/ship.png", b"\x89PNG\r\n\x1a\nbody").await;
    assert!(result.is_ok(), "a normal payload is served from the cache");
}

#[tokio::test]
async fn zero_length_file_is_currently_valid() {
    // Plan 008 will reject zero-length files; this assertion flips then.
    let (_root, result) = get_local("kcs2/resources/empty.png", b"").await;
    assert!(result.is_ok(), "an empty file is accepted today");
}

#[tokio::test]
async fn html_extension_skips_the_content_check() {
    // Plan 008 will stop exempting .html from the error-page check; this flips then.
    let (_root, result) = get_local("gadget_html5/page.html", HTML_ERROR_PAGE).await;
    assert!(result.is_ok(), ".html is exempt from the HTML error-page check today");
}

#[tokio::test]
async fn html_body_under_a_non_html_extension_is_invalid() {
    let (_root, result) = get_local("kcs2/resources/ship.png", HTML_ERROR_PAGE).await;
    // Rejected internally as InvalidFile, surfaced as FileNotFound (see module docs).
    assert!(
        matches!(result, Err(KacheError::FileNotFound(_))),
        "an error page saved as .png is not served, got {result:?}"
    );
}

#[tokio::test]
async fn bare_html_tag_without_doctype_is_also_invalid() {
    let (_root, result) =
        get_local("kcs2/resources/ship.png", b"<html><body>err</body></html>").await;
    assert!(
        matches!(result, Err(KacheError::FileNotFound(_))),
        "the check matches a bare <html tag, not just the doctype, got {result:?}"
    );
}

#[tokio::test]
async fn missing_file_is_not_found() {
    let root = TempDir::new().unwrap();
    let cache = local_only_cache(&root);
    let opt = GetOption::new().disable_remote();

    let result = cache.get_with_opt("kcs2/resources/absent.png", "", &opt).await;
    assert!(matches!(result, Err(KacheError::FileNotFound(_))));
}

#[tokio::test]
async fn directory_in_place_of_a_file_is_not_served() {
    let root = TempDir::new().unwrap();
    let rel = "kcs2/resources/dir.png";
    std::fs::create_dir_all(root.path().join(rel)).unwrap();

    let cache = local_only_cache(&root);
    let opt = GetOption::new().disable_remote();

    let result = cache.get_with_opt(rel, "", &opt).await;
    assert!(result.is_err(), "a directory never satisfies a file request");
}
