use axum::{
	extract::Request,
	middleware::{self, Next},
	response::Response,
	RequestPartsExt, Router,
};
use http::StatusCode;

use crate::{
	net::{auth::kcs_api_auth_middleware, header::add_content_type_json_header, AppState},
	state::State,
};

mod api_get_member;
mod api_port;
mod api_req_furniture;
mod api_req_hensei;
mod api_req_hokyu;
mod api_req_init;
mod api_req_kaisou;
mod api_req_kousyou;
mod api_req_member;
mod api_req_ranking;
mod api_start2;
mod api_world;

mod form_utils;

pub(super) fn router() -> Router {
	Router::new()
		// .merge(Router::new().nest("/api_dmm_payment", api_dmm_payment::router()))
		.merge(Router::new().nest("/api_get_member", api_get_member::router()))
		.merge(Router::new().nest("/api_port", api_port::router()))
		.merge(Router::new().nest("/api_req_init", api_req_init::router()))
		.merge(Router::new().nest("/api_req_furniture", api_req_furniture::router()))
		.merge(Router::new().nest("/api_req_hensei", api_req_hensei::router()))
		.merge(Router::new().nest("/api_req_hokyu", api_req_hokyu::router()))
		.merge(Router::new().nest("/api_req_kaisou", api_req_kaisou::router()))
		.merge(Router::new().nest("/api_req_kousyou", api_req_kousyou::router()))
		.merge(Router::new().nest("/api_req_member", api_req_member::router()))
		.merge(Router::new().nest("/api_req_ranking", api_req_ranking::router()))
		// .merge(Router::new().nest("/api_req_quest", api_req_quest::router()))
		.merge(Router::new().nest("/api_start2", api_start2::router()))
		.route_layer(middleware::from_fn(kcs_api_auth_middleware))
		.merge(Router::new().nest("/api_world", api_world::router()))
		.route_layer(middleware::from_fn(mocking_middleware))
		.route_layer(add_content_type_json_header())
}

pub(super) async fn mocking_middleware(
	request: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	let (mut parts, body) = request.into_parts();

	let state = parts.extract::<AppState>().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	let state: &State = state.as_ref();

	if let Some(mod_dir) = state.kache.mods_root() {
		let req_path = parts.uri.path();
		let mock_file =
			mod_dir.join("kcsapi").join(req_path.trim_start_matches('/')).with_extension("json");
		if mock_file.exists() {
			info!("🤖 mocking response for {}", req_path);
			let mock_data =
				tokio::fs::read(mock_file).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

			// check if mock_data starts with 'svdata='
			return if mock_data.starts_with(b"svdata=") {
				Ok(Response::new(mock_data.into()))
			} else {
				// append 'svdata=' to the beginning of the response
				Ok(Response::new(format!("svdata={}", String::from_utf8_lossy(&mock_data)).into()))
			};
		}
	}

	Ok(next.run(Request::from_parts(parts, body)).await)
}
