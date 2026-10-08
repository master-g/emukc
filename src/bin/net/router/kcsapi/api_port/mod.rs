use axum::{Router, routing::post};

mod air_corps_cond_recovery_with_timer;
mod port;

pub(super) fn router() -> Router {
    Router::new()
        .route("/airCorpsCondRecoveryWithTimer", post(air_corps_cond_recovery_with_timer::handler))
        .route("/port", post(port::handler))
}
