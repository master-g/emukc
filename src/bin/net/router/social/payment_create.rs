use emukc_internal::prelude::ApiMstPayitem;
use uuid::Uuid;

use crate::net::auth::GameSession;
use crate::state::{PaymentSession, State};

use super::RpcParams;

pub(super) async fn exec(
    state: &State,
    session: &GameSession,
    params: RpcParams,
) -> serde_json::Value {
    let extra = match params.params {
        Some(p) => p,
        None => return error_response("missing params"),
    };

    let items = match extra.items {
        Some(items) if !items.is_empty() => items,
        _ => return error_response("missing items"),
    };

    let item = &items[0];

    let sku_id: i64 = match item.sku_id.parse() {
        Ok(id) => id,
        Err(_) => return error_response("invalid sku_id"),
    };

    let manifest_item = match state.codex.find::<ApiMstPayitem>(&sku_id) {
        Ok(item) => item,
        Err(_) => return error_response("sku_id not found in manifest"),
    };

    let count: i64 = match item.count.parse() {
        Ok(c) => c,
        Err(_) => return error_response("invalid count"),
    };

    if count <= 0 {
        return error_response("count must be positive");
    }

    let price = manifest_item.api_price;

    let payment_id = Uuid::new_v4().to_string();
    let token = &session.token;

    let transaction_url = format!("/emukc/game/payment.html?payment_id={payment_id}&st={token}");

    let pay_session = PaymentSession {
        payment_id: payment_id.clone(),
        profile_id: session.profile.id,
        token: token.clone(),
        sku_id,
        price,
        count,
        name: item.name.clone(),
        description: item.description.clone(),
    };

    state.payment_store.insert(pay_session);

    serde_json::json!({
        "id": "key",
        "data": {
            "status": 1,
            "transactionUrl": transaction_url,
            "payment_id": payment_id,
        }
    })
}

fn error_response(msg: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "key",
        "data": {
            "status": -1,
            "msg": msg,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::GameSession;
    use emukc_internal::prelude::*;
    use std::{path::PathBuf, sync::Arc};
    use tempfile::TempDir;

    async fn setup() -> (Arc<State>, GameSession) {
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

        let account = state.sign_up("test-user", "1234567").await.unwrap();
        let profile = state.new_profile(&account.access_token.token, "test-profile").await.unwrap();
        let session =
            state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

        let game_session = GameSession {
            token: session.session.token.clone(),
            profile: session.profile.clone(),
        };

        (state, game_session)
    }

    fn valid_params(sku_id: &str, price: &str, count: &str) -> super::super::RpcParams {
        super::super::RpcParams {
            user_id: vec!["1".to_string()],
            group_id: "1".to_string(),
            app_id: None,
            fields: None,
            params: Some(super::super::RpcExtraParams {
                items: Some(vec![super::super::RpcPurchaseItem {
                    sku_id: sku_id.to_string(),
                    price: price.to_string(),
                    count: count.to_string(),
                    description: "test".to_string(),
                    name: "test item".to_string(),
                    image_url: "".to_string(),
                }]),
                payment_type: None,
                data: None,
            }),
        }
    }

    fn find_valid_sku_id(codex: &Codex) -> i64 {
        // Try common payitem IDs starting from 1
        for id in 1..100 {
            if codex.find::<ApiMstPayitem>(&id).is_ok() {
                return id;
            }
        }
        panic!("codex must have at least one payitem with id 1..100");
    }

    fn status_of(val: &serde_json::Value) -> i64 {
        val["data"]["status"].as_i64().unwrap()
    }

    #[tokio::test]
    async fn happy_path_creates_session() {
        let (state, session) = setup().await;
        let sku_id = find_valid_sku_id(&state.codex);
        let params = valid_params(&sku_id.to_string(), "500", "1");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), 1);
        assert!(result["data"]["transactionUrl"].as_str().unwrap().contains("payment_id="));
    }

    #[tokio::test]
    async fn missing_params_returns_error() {
        let (state, session) = setup().await;
        let params = super::super::RpcParams {
            user_id: vec!["1".to_string()],
            group_id: "1".to_string(),
            app_id: None,
            fields: None,
            params: None,
        };
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), -1);
    }

    #[tokio::test]
    async fn invalid_sku_id_format_returns_error() {
        let (state, session) = setup().await;
        let params = valid_params("not_a_number", "500", "1");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), -1);
    }

    #[tokio::test]
    async fn unknown_sku_id_returns_error() {
        let (state, session) = setup().await;
        let params = valid_params("99999999", "500", "1");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), -1);
    }

    #[tokio::test]
    async fn zero_count_returns_error() {
        let (state, session) = setup().await;
        let sku_id = find_valid_sku_id(&state.codex);
        let params = valid_params(&sku_id.to_string(), "500", "0");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), -1);
    }

    #[tokio::test]
    async fn negative_count_returns_error() {
        let (state, session) = setup().await;
        let sku_id = find_valid_sku_id(&state.codex);
        let params = valid_params(&sku_id.to_string(), "500", "-5");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), -1);
    }

    #[tokio::test]
    async fn manifest_price_overrides_client_price() {
        let (state, session) = setup().await;
        let sku_id = find_valid_sku_id(&state.codex);
        let manifest_item = state.codex.find::<ApiMstPayitem>(&sku_id).unwrap();

        let params = valid_params(&sku_id.to_string(), "1", "1");
        let result = exec(&state, &session, params).await;
        assert_eq!(status_of(&result), 1);

        let payment_id = result["data"]["payment_id"].as_str().unwrap();
        let stored = state.payment_store.get(payment_id).unwrap();
        assert_eq!(stored.price, manifest_item.api_price);
    }
}
