//! Version expiry tests for `Kache`.
//!
//! Unlike `version_management.rs`, which only asserts `is_ok()`, every case here
//! asserts *whether a request went out*, via the mock server's request count.
//! That is what makes the file fail if version comparison stops working — see
//! the mutation check in plan 001 step 4.

use emukc_cache::{GetOption, Kache};
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const REL_PATH: &str = "kcs2/resources/versioned.png";

/// Neither may be "1": `IntoVersion` maps "1" to `None` ("unversioned"), which
/// bypasses the comparison this file is about. See `version_one_means_unversioned`.
const OLD: &str = "6.3.0.0";
const NEW: &str = "6.3.5.0";

/// Serve a distinct body per `?version=` value so a re-download is visible in the
/// bytes as well as in the request count.
async fn versioned_server() -> MockServer {
    let server = MockServer::start().await;
    for (version, body) in [(OLD, b"body-old".as_slice()), (NEW, b"body-new".as_slice())] {
        Mock::given(method("GET"))
            .and(path(format!("/{REL_PATH}")))
            .and(query_param("version", version))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
            .mount(&server)
            .await;
    }
    server
}

fn cache_for(server: &MockServer, root: &TempDir) -> Kache {
    Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdn(server.uri())
        .with_gadgets_cdn(server.uri())
        .build()
        .unwrap()
}

async fn body_of(cache: &Kache, version: &str) -> Vec<u8> {
    let mut file = cache.get(REL_PATH, version).await.unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await.unwrap();
    buf
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.unwrap().len()
}

#[tokio::test]
async fn newer_version_forces_a_redownload_and_same_version_does_not() {
    let server = versioned_server().await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    // 1. First fetch at version 1.
    assert_eq!(body_of(&cache, OLD).await, b"body-old");
    assert_eq!(request_count(&server).await, 1);
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some(OLD));

    // 2. Version 2 is newer, so the local copy is stale and must be refetched.
    assert_eq!(body_of(&cache, NEW).await, b"body-new");
    assert_eq!(request_count(&server).await, 2, "a newer version triggers a download");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some(NEW));
    assert_eq!(std::fs::read(root.path().join(REL_PATH)).unwrap(), b"body-new");

    // 3. Version 2 again is a cache hit — no new request.
    assert_eq!(body_of(&cache, NEW).await, b"body-new");
    assert_eq!(request_count(&server).await, 2, "an unchanged version must not redownload");
}

#[tokio::test]
async fn rollback_request_keeps_the_newer_local_file() {
    let server = versioned_server().await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    body_of(&cache, NEW).await;
    let after_first = request_count(&server).await;

    // Asking for the older version 1 must not downgrade a good cache entry: the
    // newer local file is served and the recorded version is left alone.
    assert_eq!(body_of(&cache, OLD).await, b"body-new", "the newer local file is served");
    assert_eq!(request_count(&server).await, after_first, "a rollback issues no request");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some(NEW));
}

#[tokio::test]
async fn empty_version_serves_whatever_is_local() {
    let server = versioned_server().await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    body_of(&cache, OLD).await;
    let after_first = request_count(&server).await;

    // An empty version means "no specific version requested", which short-circuits
    // the comparison entirely.
    let opt = GetOption::new();
    cache.get_with_opt(REL_PATH, "", &opt).await.unwrap();
    assert_eq!(request_count(&server).await, after_first);
}

/// `IntoVersion` folds "1" into `None`, i.e. "no version". Nothing in the public
/// API hints at it, and passing "1" as if it were an ordinary version silently
/// disables both the `?version=` query and the version row.
#[tokio::test]
async fn version_one_means_unversioned() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"body".to_vec()))
        .mount(&server)
        .await;

    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    cache.get(REL_PATH, "1").await.unwrap();

    assert_eq!(
        cache.get_cached_version(REL_PATH).await.unwrap(),
        None,
        r#"version "1" records no version row"#
    );
    let request = &server.received_requests().await.unwrap()[0];
    assert!(request.url.query().is_none(), r#"version "1" sends no ?version= query"#);
}
