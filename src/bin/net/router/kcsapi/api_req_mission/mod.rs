use axum::{Router, routing::post};

mod result;
mod return_instruction;
mod start;

pub(super) fn router() -> Router {
    Router::new()
        .route("/start", post(start::handler))
        .route("/result", post(result::handler))
        .route("/return_instruction", post(return_instruction::handler))
}
