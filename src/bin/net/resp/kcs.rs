use axum::response::IntoResponse;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// API result
///
/// This is the common structure for all API responses, note that there is a `svdata=` prefix in the response body.
///
/// # Example
///
/// ```plain
/// svdata={
///   "api_result": 1,
///   "api_result_msg": "成功",
///   "api_data": {}
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiResponse {
	pub api_result: i32,
	pub api_result_msg: String,
	pub api_data: Option<Value>,
	#[serde(flatten)]
	pub extra: Option<Value>,
}

impl Default for KcApiResponse {
	fn default() -> Self {
		Self {
			api_result: 1,
			api_result_msg: "\u{6210}\u{529f}".to_string(),
			api_data: None,
			extra: None,
		}
	}
}

impl IntoResponse for KcApiResponse {
	fn into_response(self) -> axum::response::Response {
		let json = match serde_json::to_string(&self) {
			Ok(json) => format!("svdata={json}"),
			Err(_) => "svdata={\"api_result\":-1,\"api_result_msg\":\"Failed to serialize response\",\"api_data\":null}".to_string(),
		};

		(StatusCode::OK, [("Content-Type", r#"application/json;charset="utf-8""#)], json)
			.into_response()
	}
}

impl KcApiResponse {
	#[allow(dead_code)]
	pub fn success<T: Serialize>(data: &T) -> Self {
		Self {
			api_result: 1,
			api_result_msg: "\u{6210}\u{529f}".to_string(),
			api_data: Some(serde_json::to_value(data).unwrap()),
			extra: None,
		}
	}

	#[allow(dead_code)]
	pub fn success_json(data: Value) -> Self {
		Self {
			api_result: 1,
			api_result_msg: "\u{6210}\u{529f}".to_string(),
			api_data: Some(data),
			extra: None,
		}
	}

	#[allow(dead_code)]
	pub fn success_extra<T: Serialize, V: Serialize>(data: &T, extra: &Option<V>) -> Self {
		Self {
			api_result: 1,
			api_result_msg: "\u{6210}\u{529f}".to_string(),
			api_data: Some(serde_json::to_value(data).unwrap()),
			extra: extra.as_ref().map(|e| serde_json::to_value(e).unwrap()),
		}
	}

	#[allow(dead_code)]
	pub fn empty() -> Self {
		Self {
			api_result: 1,
			api_result_msg: "\u{6210}\u{529f}".to_string(),
			api_data: Some(serde_json::json!("{}")), // None,
			extra: None,
		}
	}

	#[allow(dead_code)]
	pub fn failure(msg: &str) -> Self {
		Self {
			api_result: -1,
			api_result_msg: msg.to_string(),
			api_data: None,
			extra: None,
		}
	}
}

impl From<()> for KcApiResponse {
	fn from(_: ()) -> Self {
		KcApiResponse::empty()
	}
}
