use axum::{Router, routing::post};

mod airbattle;
mod battle;
mod battle_water;
mod battleresult;
mod goback_port;
mod ld_airbattle;
mod ld_shooting;
mod midnight_battle;
mod sp_midnight;

pub(super) fn router() -> Router {
    Router::new()
        .route("/airbattle", post(airbattle::handler))
        .route("/battle", post(battle::handler))
        .route("/battle_water", post(battle_water::handler))
        .route("/battleresult", post(battleresult::handler))
        .route("/goback_port", post(goback_port::handler))
        .route("/ld_airbattle", post(ld_airbattle::handler))
        .route("/ld_shooting", post(ld_shooting::handler))
        .route("/midnight_battle", post(midnight_battle::handler))
        .route("/sp_midnight", post(sp_midnight::handler))
}
