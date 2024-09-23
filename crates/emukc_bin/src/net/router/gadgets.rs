use axum::{
	extract::Query,
	response::IntoResponse,
	routing::{get, post},
	Form, Router,
};
use emukc_internal::{model::profile::Profile, time::chrono};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::net::{header, AppState};

const UNPARSEABLE_CRUFT: &str = "throw 1; < don't be evil' >";

pub(super) fn router() -> Router {
	Router::new()
		.route("/makeRequest", get(get_make_request))
		.route("/makeRequest", post(post_make_request))
		.route_layer(header::add_content_type_json_header())
}

#[derive(Serialize, Deserialize, Debug)]
struct GetQuery {
	url: String,
}

const GET_WORLD_ID: &str = "api_world/get_id/";
const GET_INDEX_HTML: &str = "index.html";

async fn get_make_request(_state: AppState, Query(query): Query<GetQuery>) -> impl IntoResponse {
	let resp = match query.url {
		url if url.contains(GET_WORLD_ID) => {
			let _uid: i64 = url
				.split(GET_WORLD_ID)
				.nth(1)
				.unwrap_or("0")
				.split('/')
				.next()
				.unwrap_or("0")
				.parse()
				.unwrap_or(0);
			let world: Option<Profile> = None;
			// let world: Option<KcUserWorld> = if let Ok(r) = read_from(&db).one(uid).await {
			// 	r
			// } else {
			// 	return (StatusCode::INTERNAL_SERVER_ERROR, "db error".to_string());
			// };
			trace!("world: {:?}", world);
			let world_id = world.map(|w| w.world_id).unwrap_or(0);

			let api_result = serde_json::json!({
				"api_result": 1,
				"api_result_msg": "\u{6210}\u{529f}",
				"api_data": {
					"api_world_id": world_id
				}
			});
			let j = serde_json::json!({
				url: serde_json::json!({
					"rc": 200,
					"body": format!("svdata={}", api_result.to_string()),
					"headers": {
						"Server": "nginx",
						"Content-Type": "text/plain",
						"Connection": "keep-alive"
					}
				})
			});
			(StatusCode::OK, j.to_string().replace('/', r#"\/"#))
		}
		url if url.contains(GET_INDEX_HTML) => {
			let j = serde_json::json!({
				url: serde_json::json!({
					"rc": 200,
					"body": "<HTML>\r\n<BODY>\r\nKADOKAWA\r\n</BODY>\r\n</HTML>\r\n",
					"headers": {
						"Server": "nginx",
						"Content-Type": "text/html",
						"Connection": "keep-alive"
					}
				})
			});
			(StatusCode::OK, j.to_string().replace('/', r#"\/"#))
		}
		_ => (StatusCode::NOT_IMPLEMENTED, "not implemented".to_string()),
	};

	let final_resp = format!("{}{}", UNPARSEABLE_CRUFT, resp.1);
	(resp.0, final_resp)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PostFormParams {
	url: String,
	st: String,
}

const POST_DMM_LOGIN: &str = "api_auth_member/dmmlogin";

async fn post_make_request(Form(params): Form<PostFormParams>) -> impl IntoResponse {
	let resp = match params.url {
		url if url.contains(POST_DMM_LOGIN) => {
			let api_result = serde_json::json!({
				"api_result": 1,
				"api_result_msg": "\u{6210}\u{529f}",
				"api_token": params.st,
				"api_starttime": chrono::Utc::now().timestamp_millis(),
			});
			let j = serde_json::json!({
				url: serde_json::json!({
					"rc": 200,
					"body": format!("svdata={}", api_result.to_string()),
					"headers": {
						"Server": "nginx",
						"Content-Type": "text/plain",
						"Connection": "keep-alive"
					}
				})
			});
			(StatusCode::OK, j.to_string().replace('/', r#"\/"#))
		}
		_ => (StatusCode::NOT_IMPLEMENTED, "not implemented".to_string()),
	};

	let final_resp = format!("{}{}", UNPARSEABLE_CRUFT, resp.1);
	(resp.0, final_resp)
}
