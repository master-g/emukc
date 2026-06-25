use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use axum::{
    RequestPartsExt, Router,
    body::{Body, to_bytes},
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use emukc::cache::{GetOption, NoVersion};
use emukc_internal::time::chrono::Utc;
use http::{Method, StatusCode};
use tokio::io::AsyncReadExt;

use crate::{
    net::{AppState, auth::kcs_api_auth_middleware, header::add_content_type_json_header},
    state::State,
};

mod api_dmm_payment;
mod api_get_member;
mod api_port;
mod api_req_battle_midnight;
mod api_req_furniture;
mod api_req_hensei;
mod api_req_hokyu;
mod api_req_init;
mod api_req_kaisou;
mod api_req_kousyou;
mod api_req_map;
mod api_req_member;
mod api_req_mission;
mod api_req_nyukyo;
mod api_req_practice;
mod api_req_quest;
mod api_req_ranking;
mod api_req_sortie;
mod api_start2;
mod api_world;

mod form_utils;

pub(super) fn router() -> Router {
    Router::new()
        .merge(Router::new().nest("/api_dmm_payment", api_dmm_payment::router()))
        .merge(Router::new().nest("/api_get_member", api_get_member::router()))
        .merge(Router::new().nest("/api_port", api_port::router()))
        .merge(Router::new().nest("/api_req_init", api_req_init::router()))
        .merge(Router::new().nest("/api_req_furniture", api_req_furniture::router()))
        .merge(Router::new().nest("/api_req_battle_midnight", api_req_battle_midnight::router()))
        .merge(Router::new().nest("/api_req_hensei", api_req_hensei::router()))
        .merge(Router::new().nest("/api_req_hokyu", api_req_hokyu::router()))
        .merge(Router::new().nest("/api_req_kaisou", api_req_kaisou::router()))
        .merge(Router::new().nest("/api_req_kousyou", api_req_kousyou::router()))
        .merge(Router::new().nest("/api_req_map", api_req_map::router()))
        .merge(Router::new().nest("/api_req_member", api_req_member::router()))
        .merge(Router::new().nest("/api_req_mission", api_req_mission::router()))
        .merge(Router::new().nest("/api_req_nyukyo", api_req_nyukyo::router()))
        .merge(Router::new().nest("/api_req_practice", api_req_practice::router()))
        .merge(Router::new().nest("/api_req_ranking", api_req_ranking::router()))
        .merge(Router::new().nest("/api_req_quest", api_req_quest::router()))
        .merge(Router::new().nest("/api_req_sortie", api_req_sortie::router()))
        .merge(Router::new().nest("/api_start2", api_start2::router()))
        .route_layer(middleware::from_fn(kcs_api_auth_middleware))
        .merge(Router::new().nest("/api_world", api_world::router()))
        .route_layer(middleware::from_fn(mocking_middleware))
        .route_layer(add_content_type_json_header())
        .route_layer(middleware::from_fn(dump_middleware))
}

pub(super) async fn mocking_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (mut parts, body) = request.into_parts();

    let state = parts.extract::<AppState>().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let state: &State = state.as_ref();

    let req_path = parts.uri.path();
    let mock_path = format!("kcsapi{req_path}.json");
    if let Ok(mut f) = GetOption::new_api_mocking().get(&state.kache, &mock_path, NoVersion).await {
        info!("🤖 mocking response for {}", req_path);
        let mut raw = String::new();
        f.read_to_string(&mut raw).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // check if mock_data starts with 'svdata='
        return if raw.starts_with("svdata=") {
            Ok(Response::new(raw.into()))
        } else {
            // append 'svdata=' to the beginning of the response
            Ok(Response::new(format!("svdata={raw}").into()))
        };
    }

    Ok(next.run(Request::from_parts(parts, body)).await)
}

/// Max body size (request or response) captured per record. Sortie battle
/// payloads are the largest KCSAPI bodies and stay well under this.
const DUMP_BODY_LIMIT: usize = 32 * 1024 * 1024;

/// JSONL request/response capture for KCSAPI traffic, gated by the
/// `EMUKC_KCSAPI_DUMP` env var so it is off — and zero-cost — by default.
///
/// - unset / empty: disabled, the middleware is a passthrough.
/// - `1` / `on` / `true`: write to `.data/logs/kcsapi_dump.jsonl`.
/// - any other value: treated as the output file path.
///
/// Each line is one `{ts,method,path,query,request,status,response}` record. The
/// `request` is the raw form body and `response` is the raw `svdata=`-prefixed
/// body (uncompressed — this middleware sits inside the outer compression layer).
///
/// ponytail: single-user dev tool — env read + blocking append per request, no
/// rotation or locking. Swap in a background writer + size cap if it ever needs
/// multi-client or high-volume capture.
pub(super) async fn dump_middleware(request: Request, next: Next) -> Response {
    let Some(target) = dump_target() else {
        return next.run(request).await;
    };

    let (parts, body) = request.into_parts();
    let method = parts.method.clone();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().unwrap_or_default().to_string();
    let req_bytes = to_bytes(body, DUMP_BODY_LIMIT).await.unwrap_or_default();
    let request = Request::from_parts(parts, Body::from(req_bytes.clone()));

    let response = next.run(request).await;
    let (parts, body) = response.into_parts();
    let status = parts.status;
    let resp_bytes = to_bytes(body, DUMP_BODY_LIMIT).await.unwrap_or_default();
    let response = Response::from_parts(parts, Body::from(resp_bytes.clone()));

    let record = format_dump_record(
        &method,
        &path,
        &query,
        &String::from_utf8_lossy(&req_bytes),
        status,
        &String::from_utf8_lossy(&resp_bytes),
    );
    if let Err(e) = append_line(&target, &record) {
        warn!("failed to write kcsapi dump to {}: {e}", target.display());
    }

    response
}

/// Resolve the dump output path from `EMUKC_KCSAPI_DUMP`, or `None` when disabled.
fn dump_target() -> Option<PathBuf> {
    parse_dump_target(std::env::var("EMUKC_KCSAPI_DUMP").ok()?.as_str())
}

fn parse_dump_target(value: &str) -> Option<PathBuf> {
    match value.trim() {
        "" => None,
        "1" | "on" | "true" => Some(PathBuf::from(".data/logs/kcsapi_dump.jsonl")),
        path => Some(PathBuf::from(path)),
    }
}

fn format_dump_record(
    method: &Method,
    path: &str,
    query: &str,
    request: &str,
    status: StatusCode,
    response: &str,
) -> String {
    serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "method": method.as_str(),
        "path": path,
        "query": query,
        "request": request,
        "status": status.as_u16(),
        "response": response,
    })
    .to_string()
}

fn append_line(target: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(target)?;
    writeln!(file, "{line}")
}

#[cfg(test)]
mod dump_tests {
    use super::*;

    #[test]
    fn parse_dump_target_semantics() {
        assert_eq!(parse_dump_target(""), None);
        assert_eq!(parse_dump_target("   "), None);
        assert_eq!(parse_dump_target("1"), Some(PathBuf::from(".data/logs/kcsapi_dump.jsonl")));
        assert_eq!(parse_dump_target("on"), Some(PathBuf::from(".data/logs/kcsapi_dump.jsonl")));
        assert_eq!(parse_dump_target("/tmp/x.jsonl"), Some(PathBuf::from("/tmp/x.jsonl")));
    }

    #[test]
    fn format_dump_record_is_valid_jsonl() {
        let line = format_dump_record(
            &Method::POST,
            "/kcsapi/api_req_map/next",
            "",
            "api_token=abc&api_cell_id=4",
            StatusCode::OK,
            "svdata={\"api_result\":1}",
        );
        assert!(!line.contains('\n'), "record must be a single JSONL line");
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["method"], "POST");
        assert_eq!(v["path"], "/kcsapi/api_req_map/next");
        assert_eq!(v["status"], 200);
        assert_eq!(v["request"], "api_token=abc&api_cell_id=4");
        assert_eq!(v["response"], "svdata={\"api_result\":1}");
        assert!(v["ts"].as_str().is_some_and(|s| !s.is_empty()));
    }
}

#[cfg(test)]
pub(super) mod test_utils {
    use super::*;
    use axum::Extension;
    use emukc_internal::prelude::*;
    use std::{path::PathBuf, sync::Arc};
    use tempfile::TempDir;

    use crate::{net::auth::GameSession, state::State};

    pub(super) struct TestContext {
        #[expect(dead_code)]
        pub cache_root: TempDir,
        pub state: Arc<State>,
        pub session: GameSession,
    }

    pub(super) async fn new_test_context() -> TestContext {
        let cache_root = tempfile::tempdir().unwrap();
        let db = Arc::new(new_mem_db().await.unwrap());
        let codex = Codex::load_without_cache_source(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex"),
        )
        .unwrap();
        let cache_path = cache_root.path().join("cache");
        std::fs::create_dir_all(&cache_path).unwrap();
        let kache = Arc::new(
            Kache::builder()
                .with_cache_root(cache_path)
                .with_gadgets_cdn("https://example.invalid/gadgets".to_string())
                .with_content_cdn("https://example.invalid/content".to_string())
                .build()
                .unwrap(),
        );
        let state = Arc::new(State {
            db,
            kache,
            codex: Arc::new(codex),
            sortie_store: Arc::new(SortieStore::new()),
            practice_store: Arc::new(PracticeStore::new()),
            payment_store: Arc::new(crate::state::PaymentStore::new()),
        });

        let account = state.sign_up("router-test", "1234567").await.unwrap();
        let profile = state.new_profile(&account.access_token.token, "router-admin").await.unwrap();
        let session =
            state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

        TestContext {
            cache_root,
            state,
            session: GameSession {
                token: session.session.token.clone(),
                profile: session.profile.clone(),
            },
        }
    }

    pub(super) async fn seed_single_ship_fleet(state: &Arc<State>, profile_id: i64) {
        let ship = state.add_ship(profile_id, 951).await.unwrap();
        state.update_fleet_ships(profile_id, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
    }

    pub(super) fn app_state(state: &Arc<State>) -> AppState {
        Extension(state.clone())
    }
}
