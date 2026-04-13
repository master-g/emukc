use axum::{Router, routing::post};

mod battle;
mod battle_result;
mod midnight_battle;

pub(super) fn router() -> Router {
    Router::new()
        .route("/battle", post(battle::handler))
        .route("/battle_result", post(battle_result::handler))
        .route("/midnight_battle", post(midnight_battle::handler))
}
