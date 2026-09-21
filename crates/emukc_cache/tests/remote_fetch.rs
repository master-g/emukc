//! Remote fetch integration tests for `Kache`.
//!
//! Baseline for plan 003, and the corrected form of what plan 008 flipped:
//! including the parts that are known to be wrong. Assertions that a later plan
//! is expected to flip carry a comment saying so. Plan 007 has landed: a 404 and
//! a total CDN failure are now distinct outcomes, pinned below.

use emukc_cache::{GetOption, Kache, KacheError};
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const REL_PATH: &str = "kcs2/resources/test.png";

/// Not "1": `IntoVersion` maps "1" (and 0/1/"") to `None`, meaning "unversioned",
/// which skips both the `?version=` query and the recorded version row.
const VERSION: &str = "6.3.5.0";

/// A cache rooted in a temp dir whose only CDN is `server`.
fn cache_for(server: &MockServer, root: &TempDir) -> Kache {
    Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdn(server.uri())
        .with_gadgets_cdn(server.uri())
        .build()
        .unwrap()
}

async fn read_back(file: &mut tokio::fs::File) -> Vec<u8> {
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await.unwrap();
    buf
}

#[tokio::test]
async fn fetch_200_writes_body_and_records_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"png-bytes".to_vec()))
        .mount(&server)
        .await;

    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    let mut file = cache.get(REL_PATH, VERSION).await.unwrap();
    assert_eq!(read_back(&mut file).await, b"png-bytes");
    assert_eq!(std::fs::read(root.path().join(REL_PATH)).unwrap(), b"png-bytes");

    // One request, not two: this is the regression guard for plan 003's
    // skip_header_check(true). Without it the download layer probes with a HEAD
    // before the GET, and the count here goes to 2.
    assert_eq!(server.received_requests().await.unwrap().len(), 1, "one GET, no HEAD probe");

    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some(VERSION));

    // Asking for the same version again must be served locally, with no new request.
    cache.get(REL_PATH, VERSION).await.unwrap();
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "second get at the same version hits the local cache"
    );
}

#[tokio::test]
async fn fetch_200_with_empty_body_fails_and_leaves_no_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(Vec::new()))
        .mount(&server)
        .await;

    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    let err = cache.get(REL_PATH, VERSION).await.unwrap_err();
    assert!(matches!(err, KacheError::FailedOnAllCdn), "got {err:?}");
    // The empty file must not survive the rejection: for a path with no version
    // `find_in_local` would otherwise serve it back on the next run.
    assert!(!root.path().join(REL_PATH).exists(), "an empty body leaves nothing on disk");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap(), None, "no version recorded");
}

#[tokio::test]
async fn fetch_404_fails_and_leaves_no_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    let err = cache.get(REL_PATH, VERSION).await.unwrap_err();
    // A 404 is the CDN answering "this does not exist", which is a different
    // outcome from "no CDN gave an answer" — see `fetch_fails_when_every_cdn_errors`.
    assert!(matches!(err, KacheError::FileNotFound(_)), "got {err:?}");
    assert!(!root.path().join(REL_PATH).exists(), "404 leaves nothing on disk");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap(), None, "no version recorded");

    // Nothing was recorded, so a retry goes back out to the CDN.
    let _ = cache.get(REL_PATH, VERSION).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn fetch_fails_when_every_cdn_errors() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    for server in [&first, &second] {
        Mock::given(method("GET"))
            .and(path(format!("/{REL_PATH}")))
            .respond_with(ResponseTemplate::new(500))
            .mount(server)
            .await;
    }

    let root = TempDir::new().unwrap();
    let cache = Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdns(vec![first.uri(), second.uri()])
        .with_gadgets_cdn(first.uri())
        .build()
        .unwrap();

    let opt = GetOption::new();
    let err = cache.get_with_opt(REL_PATH, VERSION, &opt).await.unwrap_err();
    assert!(matches!(err, KacheError::FailedOnAllCdn), "got {err:?}");
    assert!(!root.path().join(REL_PATH).exists());

    // A 500 is retried on the next CDN, unlike the 404 above which stops at the first.
    assert_eq!(first.received_requests().await.unwrap().len(), 1);
    assert_eq!(second.received_requests().await.unwrap().len(), 1);
}

/// The flip side of the recovery case below: a 404 is treated as an answer and
/// ends the walk, so `FileNotFound` only ever reflects the mirror that answered.
/// Pinning this keeps the error's meaning honest — if the walk is ever changed to
/// poll every mirror before concluding absence, this test says so out loud.
#[tokio::test]
async fn fetch_404_stops_the_walk_without_asking_the_next_cdn() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&first)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"png-bytes".to_vec()))
        .mount(&second)
        .await;

    let root = TempDir::new().unwrap();
    let cache = Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdns(vec![first.uri(), second.uri()])
        .with_gadgets_cdn(first.uri())
        .build()
        .unwrap();

    let opt = GetOption::new().diable_shuffle();
    let err = cache.get_with_opt(REL_PATH, VERSION, &opt).await.unwrap_err();

    assert!(matches!(err, KacheError::FileNotFound(_)), "got {err:?}");
    assert_eq!(first.received_requests().await.unwrap().len(), 1);
    assert_eq!(
        second.received_requests().await.unwrap().len(),
        0,
        "the 404 ended the walk, so the mirror that has the file was never asked"
    );
}

#[tokio::test]
async fn fetch_recovers_when_a_failing_cdn_is_followed_by_a_working_one() {
    let failing = MockServer::start().await;
    let working = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(500))
        .mount(&failing)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/{REL_PATH}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"png-bytes".to_vec()))
        .mount(&working)
        .await;

    let root = TempDir::new().unwrap();
    let cache = Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdns(vec![failing.uri(), working.uri()])
        .with_gadgets_cdn(failing.uri())
        .build()
        .unwrap();

    // Shuffle off so the failing CDN is always tried first; with it on, the
    // working CDN could be picked first and the recovery path would go untested.
    let opt = GetOption::new().diable_shuffle();
    let mut file = cache.get_with_opt(REL_PATH, VERSION, &opt).await.unwrap();

    assert_eq!(read_back(&mut file).await, b"png-bytes");
    assert_eq!(failing.received_requests().await.unwrap().len(), 1, "the 500 was tried");
    assert_eq!(working.received_requests().await.unwrap().len(), 1, "and the loop moved on");
}
