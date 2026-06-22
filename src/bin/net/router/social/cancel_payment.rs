use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::net::{AppState, auth::GameSession};
use crate::state::State;

#[derive(Debug, Deserialize)]
pub(super) struct CancelQuery {
    payment_id: String,
    #[allow(dead_code)]
    st: Option<String>,
}

#[derive(Debug, Serialize)]
struct CancelResponse {
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
    axum::extract::Query(query): axum::extract::Query<CancelQuery>,
) -> Json<serde_json::Value> {
    let state: &State = state.as_ref();
    let payment_id = query.payment_id.clone();

    match state.payment_store.get(&payment_id) {
        Some(session_data) if session_data.profile_id != session.profile.id => {
            return Json(serde_json::json!(ErrorResponse {
                response_code: "ERROR".to_string(),
                msg: "profile mismatch".to_string(),
            }));
        }
        Some(_) => {
            state.payment_store.take(&payment_id);
        }
        None => {}
    }

    Json(serde_json::json!(CancelResponse {
        response_code: "CANCEL".to_string(),
        payment_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PaymentSession;
    use emukc_internal::prelude::*;
    use std::sync::Arc;

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

        let account = state.sign_up("cancel-test", "1234567").await.unwrap();
        let profile =
            state.new_profile(&account.access_token.token, "cancel-profile").await.unwrap();
        let session =
            state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

        let game_session = GameSession {
            token: session.session.token.clone(),
            profile: session.profile.clone(),
        };

        (state, game_session)
    }

    async fn setup_second_user(state: &State) -> GameSession {
        let account = state.sign_up("cancel-other", "1234567").await.unwrap();
        let profile =
            state.new_profile(&account.access_token.token, "cancel-other-profile").await.unwrap();
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
        let query = axum::extract::Query(CancelQuery {
            payment_id: payment_id.to_string(),
            st: None,
        });
        handler(state_ext, session_ext, query).await.0
    }

    #[tokio::test]
    async fn happy_path_cancels_and_consumes_session() {
        let (state, session) = setup().await;
        state.payment_store.insert(test_session("cancel-1", session.profile.id));

        let result = call_handler(&state, &session, "cancel-1").await;
        assert_eq!(response_code(&result), "CANCEL");
        assert!(state.payment_store.get("cancel-1").is_none());
    }

    #[tokio::test]
    async fn profile_mismatch_preserves_session() {
        let (state, session) = setup().await;
        let other = setup_second_user(&state).await;

        state.payment_store.insert(test_session("cancel-2", session.profile.id));

        let result = call_handler(&state, &other, "cancel-2").await;
        assert_eq!(response_code(&result), "ERROR");
        assert!(state.payment_store.get("cancel-2").is_some());
    }

    #[tokio::test]
    async fn nonexistent_payment_returns_cancel() {
        let (state, session) = setup().await;
        let result = call_handler(&state, &session, "nonexistent").await;
        assert_eq!(response_code(&result), "CANCEL");
    }
}
