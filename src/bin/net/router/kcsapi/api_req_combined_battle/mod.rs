use axum::{Router, routing::post};

mod battle;
mod battle_water;
mod battleresult;
mod goback_port;
mod midnight_battle;

pub(super) fn router() -> Router {
    Router::new()
        .route("/battle", post(battle::handler))
        .route("/battle_water", post(battle_water::handler))
        .route("/battleresult", post(battleresult::handler))
        .route("/goback_port", post(goback_port::handler))
        .route("/midnight_battle", post(midnight_battle::handler))
}
