//! Resource history: what a version bump keeps, and when it downloads at all.
//!
//! See `docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (L0).

use emukc_cache::{GetOption, Kache};
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const REL_PATH: &str = "kcs2/resources/ship/full/0009_5991_test.png";

fn cache_for(server: &MockServer, root: &TempDir) -> Kache {
    Kache::builder()
        .with_cache_root(root.path().to_path_buf())
        .with_content_cdn(server.uri())
        .with_gadgets_cdn(server.uri())
        .build()
        .unwrap()
}

async fn serve(server: &MockServer, verb: &str, version: &str, body: &[u8]) {
    Mock::given(method(verb))
        .and(path(format!("/{REL_PATH}")))
        .and(query_param("version", version))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
        .mount(server)
        .await;
}

async fn read(cache: &Kache, version: &str, opt: &GetOption) -> Vec<u8> {
    let mut buf = Vec::new();
    cache.get_with_opt(REL_PATH, version, opt).await.unwrap().read_to_end(&mut buf).await.unwrap();
    buf
}

async fn requests(server: &MockServer, verb: &str) -> usize {
    let requests = server.received_requests().await.unwrap();
    requests.iter().filter(|request| request.method.as_str() == verb).count()
}

fn local_only() -> GetOption {
    GetOption::new().disable_remote().disable_mod()
}

#[tokio::test]
async fn a_new_version_keeps_the_one_it_replaces() {
    let server = MockServer::start().await;
    serve(&server, "GET", "59", b"normal art").await;
    serve(&server, "HEAD", "60", b"festival art, longer").await;
    serve(&server, "GET", "60", b"festival art, longer").await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    assert_eq!(read(&cache, "59", &GetOption::new()).await, b"normal art");
    assert_eq!(read(&cache, "60", &GetOption::new()).await, b"festival art, longer");

    // The live file is the new one, and the old one is still asked for by its version.
    assert_eq!(std::fs::read(root.path().join(REL_PATH)).unwrap(), b"festival art, longer");
    assert_eq!(read(&cache, "59", &local_only()).await, b"normal art");
    assert_eq!(read(&cache, "60", &local_only()).await, b"festival art, longer");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some("60"));
}

#[tokio::test]
async fn a_version_bump_of_the_same_size_is_not_downloaded() {
    let server = MockServer::start().await;
    serve(&server, "GET", "909", b"port voice").await;
    // Same length on the CDN: an identical re-upload, as 655 port voices were on 2026-10-14.
    serve(&server, "HEAD", "1014", b"port voice").await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);

    read(&cache, "909", &GetOption::new()).await;
    assert_eq!(read(&cache, "1014", &GetOption::new()).await, b"port voice");

    assert_eq!(requests(&server, "GET").await, 1, "only the first version is downloaded");
    assert_eq!(requests(&server, "HEAD").await, 1);
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some("1014"));
    // Both versions resolve without the CDN.
    assert_eq!(read(&cache, "909", &local_only()).await, b"port voice");
    assert_eq!(read(&cache, "1014", &local_only()).await, b"port voice");
}

#[tokio::test]
async fn a_script_is_downloaded_on_every_version_bump() {
    const SCRIPT: &str = "kcs2/js/main.js";
    let server = MockServer::start().await;
    // As long as the old one: a fix that changed a digit.
    for (verb, version, body) in [
        ("GET", "6.3.6.0", "main.js of 6.3.6.0"),
        ("HEAD", "6.3.7.0", "main.js of 6.3.7.0"),
        ("GET", "6.3.7.0", "main.js of 6.3.7.0"),
    ] {
        Mock::given(method(verb))
            .and(path(format!("/{SCRIPT}")))
            .and(query_param("version", version))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
    }
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);
    let read = async |version, opt| {
        let mut buf = String::new();
        cache
            .get_with_opt(SCRIPT, version, &opt)
            .await
            .unwrap()
            .read_to_string(&mut buf)
            .await
            .unwrap();
        buf
    };

    read("6.3.6.0", GetOption::new()).await;
    assert_eq!(read("6.3.7.0", GetOption::new()).await, "main.js of 6.3.7.0");

    assert_eq!(requests(&server, "GET").await, 2, "the bump is downloaded");
    assert_eq!(requests(&server, "HEAD").await, 0, "without asking for its length");
    assert_eq!(read("6.3.6.0", local_only()).await, "main.js of 6.3.6.0");
}

#[tokio::test]
async fn an_old_version_gets_the_content_it_had_then() {
    let server = MockServer::start().await;
    serve(&server, "GET", "50", b"first").await;
    serve(&server, "HEAD", "55", b"second!").await;
    serve(&server, "GET", "55", b"second!").await;
    serve(&server, "HEAD", "60", b"third!!!").await;
    serve(&server, "GET", "60", b"third!!!").await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);
    for version in ["50", "55", "60"] {
        read(&cache, version, &GetOption::new()).await;
    }

    // Between two recorded versions: the earlier one was what the CDN served then.
    assert_eq!(read(&cache, "57", &local_only()).await, b"second!");
    // Before the first record: the earliest content there is.
    assert_eq!(read(&cache, "40", &local_only()).await, b"first");
}

#[tokio::test]
async fn an_imported_snapshot_answers_for_its_version() {
    let server = MockServer::start().await;
    serve(&server, "GET", "60", b"festival art").await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);
    read(&cache, "60", &GetOption::new()).await;

    let snapshot = root.path().join("snapshot.png");
    std::fs::write(&snapshot, b"normal art").unwrap();
    cache.keep_version(REL_PATH, "59", &snapshot, None).await.unwrap();

    assert_eq!(read(&cache, "59", &local_only()).await, b"normal art");
    // The live file and its recorded version are untouched.
    assert_eq!(read(&cache, "60", &local_only()).await, b"festival art");
    assert_eq!(cache.get_cached_version(REL_PATH).await.unwrap().as_deref(), Some("60"));
}

#[tokio::test]
async fn a_live_file_with_no_recorded_version_is_downloaded_whatever_its_size() {
    let server = MockServer::start().await;
    serve(&server, "HEAD", "60", b"same size!").await;
    serve(&server, "GET", "60", b"same size!").await;
    let root = TempDir::new().unwrap();
    let cache = cache_for(&server, &root);
    // A file of unknown origin: on disk, never recorded.
    let local = root.path().join(REL_PATH);
    std::fs::create_dir_all(local.parent().unwrap()).unwrap();
    std::fs::write(&local, b"who knows!").unwrap();

    assert_eq!(read(&cache, "60", &GetOption::new()).await, b"same size!");
    assert_eq!(requests(&server, "HEAD").await, 0);
    assert_eq!(requests(&server, "GET").await, 1);
}
