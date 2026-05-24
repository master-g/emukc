use axum::{Extension, Json};
use emukc_internal::prelude::PayItemOps;
use serde::{Deserialize, Serialize};

use crate::net::{AppState, auth::GameSession};
use crate::state::State;

#[derive(Debug, Deserialize)]
pub(super) struct ConfirmQuery {
    payment_id: String,
    #[allow(dead_code)]
    st: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfirmResponse {
    response_code: String,
    payment_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    response_code: String,
    msg: String,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    axum::extract::Query(query): axum::extract::Query<ConfirmQuery>,
) -> Json<serde_json::Value> {
    let state: &State = state.as_ref();

    let session_data = match state.payment_store.get(&query.payment_id) {
        Some(s) => s,
        None => {
            return Json(serde_json::json!(ErrorResponse {
                response_code: "ERROR".to_string(),
                msg: "payment session not found".to_string(),
            }));
        }
    };

    if session_data.profile_id != session.profile.id {
        return Json(serde_json::json!(ErrorResponse {
            response_code: "ERROR".to_string(),
            msg: "profile mismatch".to_string(),
        }));
    }

    let session_data = state.payment_store.take(&query.payment_id).unwrap();

    if let Err(_e) =
        state.add_pay_item(session.profile.id, session_data.sku_id, session_data.count).await
    {
        return Json(serde_json::json!(ErrorResponse {
            response_code: "ERROR".to_string(),
            msg: "failed to add item".to_string(),
        }));
    }

    Json(serde_json::json!(ConfirmResponse {
        response_code: "OK".to_string(),
        payment_id: query.payment_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PaymentSession;
    use emukc_internal::prelude::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn setup() -> (Arc<State>, GameSession) {
        let cache_root = tempfile::tempdir().unwrap();
        let db = Arc::new(new_mem_db().await.unwrap());
        let codex = Codex::load_without_cache_source(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex"),
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

        let account = state.sign_up("confirm-test", "1234567").await.unwrap();
        let profile =
            state.new_profile(&account.access_token.token, "confirm-profile").await.unwrap();
        let session =
            state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

        let game_session = GameSession {
            token: session.session.token.clone(),
            profile: session.profile.clone(),
        };

        (state, game_session)
    }

    async fn setup_second_user(state: &State) -> GameSession {
        let account = state.sign_up("confirm-other", "1234567").await.unwrap();
        let profile =
            state.new_profile(&account.access_token.token, "confirm-other-profile").await.unwrap();
        let session =
            state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        GameSession {
            token: session.session.token.clone(),
            profile: session.profile.clone(),
        }
    }

    fn test_session(payment_id: &str, profile_id: i64) -> PaymentSession {
        PaymentSession {
            payment_id: payment_id.to_string(),
            profile_id,
            token: "tok".to_string(),
            sku_id: 100,
            price: 500,
            count: 1,
            name: "Test".to_string(),
            description: "Test".to_string(),
        }
    }

    fn response_code(val: &serde_json::Value) -> &str {
        val["response_code"].as_str().unwrap()
    }

    async fn call_handler(
        state: &Arc<State>,
        session: &GameSession,
        payment_id: &str,
    ) -> serde_json::Value {
        let state_ext: AppState = Extension(state.clone());
        let session_ext: Extension<GameSession> = Extension(session.clone());
        let query = axum::extract::Query(ConfirmQuery {
            payment_id: payment_id.to_string(),
            st: None,
        });
        handler(state_ext, session_ext, query).await.0
    }

    #[tokio::test]
    async fn happy_path_confirms_and_consumes_session() {
        let (state, session) = setup().await;
        state.payment_store.insert(test_session("pay-1", session.profile.id));

        let result = call_handler(&state, &session, "pay-1").await;
        assert_eq!(response_code(&result), "OK");
        assert!(state.payment_store.get("pay-1").is_none());
    }

    #[tokio::test]
    async fn profile_mismatch_preserves_session() {
        let (state, session) = setup().await;
        let other = setup_second_user(&state).await;

        state.payment_store.insert(test_session("pay-2", session.profile.id));

        let result = call_handler(&state, &other, "pay-2").await;
        assert_eq!(response_code(&result), "ERROR");
        assert!(state.payment_store.get("pay-2").is_some());
    }

    #[tokio::test]
    async fn missing_session_returns_error() {
        let (state, session) = setup().await;
        let result = call_handler(&state, &session, "nonexistent").await;
        assert_eq!(response_code(&result), "ERROR");
    }

    #[tokio::test]
    async fn double_confirm_second_fails() {
        let (state, session) = setup().await;
        state.payment_store.insert(test_session("pay-3", session.profile.id));

        let first = call_handler(&state, &session, "pay-3").await;
        assert_eq!(response_code(&first), "OK");

        let second = call_handler(&state, &session, "pay-3").await;
        assert_eq!(response_code(&second), "ERROR");
    }
}
