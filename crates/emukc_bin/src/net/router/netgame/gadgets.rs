use axum::{Form, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct GadgetsRequest {
	app_id: i64,
	act: String,
	st: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct GadgetsResponse {
	status: String,
	result: String,
	time: f64,
}

pub(super) async fn handler(Form(req): Form<GadgetsRequest>) -> Json<GadgetsResponse> {
	let time = chrono::Utc::now().timestamp_millis() as f64;
	let time = time / 1000.0;

	Json(GadgetsResponse {
		status: "ok".to_string(),
		result: req.st,
		time,
	})
}
