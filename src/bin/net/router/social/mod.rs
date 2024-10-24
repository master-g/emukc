use axum::{middleware, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::{kcs_api_auth_middleware, GameSession},
	AppState,
};

mod inspection_create;
mod people_get;

const METHOD_INSPECTION_CREATE: &str = "inspection.create";
const METHOD_PEOPLE_GET: &str = "people.get";

pub(super) fn router() -> Router {
	Router::new().route("/rpc", post(rpc)).route_layer(middleware::from_fn(kcs_api_auth_middleware))
}

type RpcRequest = Vec<RpcRequestElement>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcRequestElement {
	pub method: String,
	pub params: RpcParams,
	pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcParams {
	pub user_id: Vec<String>,
	pub group_id: String,
	pub app_id: Option<String>,
	pub fields: Option<Vec<String>>,
	pub params: Option<RpcExtraParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcExtraParams {
	pub items: Option<Vec<RpcPurchaseItem>>,
	pub payment_type: Option<String>,
	pub data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPurchaseItem {
	pub sku_id: String,
	pub price: String,
	pub count: String,
	pub description: String,
	pub name: String,
	pub image_url: String,
}

async fn rpc(
	state: AppState,
	Extension(session): Extension<GameSession>,
	axum::extract::Json(req): axum::extract::Json<RpcRequest>,
) -> Json<serde_json::Value> {
	let mut resp: Vec<serde_json::Value> = Vec::new();

	for req_element in req {
		let resp_element = match req_element.method.as_str() {
			METHOD_INSPECTION_CREATE => {
				inspection_create::exec(state.as_ref(), &session, req_element.params).await
			}
			METHOD_PEOPLE_GET => {
				people_get::exec(state.as_ref(), &session, req_element.params).await
			}
			_ => serde_json::json!(
				{
					"msg": "method not implemented",
					"status": -1,
				}
			),
		};
		resp.push(resp_element);
	}

	Json(serde_json::json!(resp))
}
