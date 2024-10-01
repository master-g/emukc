use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::net::resp::KcApiResponse;

#[derive(Serialize, Deserialize, Debug)]
struct WorldInfo {
	api_id: i32,
	api_entry: i32,
	api_rate: f64,
}

pub(super) async fn handler() -> impl IntoResponse {
	let world_list = (1..=20)
		.map(|i| WorldInfo {
			api_id: i,
			api_entry: 1,
			api_rate: ((i - 1) % 9 + 1) as f64 / 10.0,
		})
		.collect::<Vec<_>>();
	let world_list = serde_json::json!(world_list);

	let data = serde_json::json!({
		"api_world_info": world_list,
	});

	KcApiResponse::success_json(data)
}
