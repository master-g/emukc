use axum::{
	RequestPartsExt, Router,
	extract::Request,
	middleware::{self, Next},
	response::Response,
};
use emukc::cache::{GetOption, NoVersion};
use http::StatusCode;
use tokio::io::AsyncReadExt;

use crate::{
	net::{AppState, auth::kcs_api_auth_middleware, header::add_content_type_json_header},
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
mod api_req_nyukyo;
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
		.merge(Router::new().nest("/api_req_nyukyo", api_req_nyukyo::router()))
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

	let req_path = parts.uri.path();
	let mock_path = format!("kcsapi{}.json", req_path);
	if let Ok(mut f) = GetOption::new_api_mocking().get(&state.kache, &mock_path, NoVersion).await {
		info!("🤖 mocking response for {}", req_path);
		let mut raw = String::new();
		f.read_to_string(&mut raw).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

		// check if mock_data starts with 'svdata='
		return if raw.starts_with("svdata=") {
			Ok(Response::new(raw.into()))
		} else {
			// append 'svdata=' to the beginning of the response
			Ok(Response::new(format!("svdata={}", raw).into()))
		};
	}

	Ok(next.run(Request::from_parts(parts, body)).await)
}
