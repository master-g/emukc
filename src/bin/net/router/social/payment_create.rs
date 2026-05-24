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

    let price: i64 = match item.price.parse() {
        Ok(p) => p,
        Err(_) => return error_response("invalid price"),
    };

    let count: i64 = match item.count.parse() {
        Ok(c) => c,
        Err(_) => return error_response("invalid count"),
    };

    // Validate sku_id against codex
    if state.codex.find::<ApiMstPayitem>(&sku_id).is_err() {
        return error_response("sku_id not found in manifest");
    }

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
