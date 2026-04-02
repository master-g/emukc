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
mod api_req_battle_midnight;
mod api_req_furniture;
mod api_req_hensei;
mod api_req_hokyu;
mod api_req_init;
mod api_req_kaisou;
mod api_req_kousyou;
mod api_req_map;
mod api_req_member;
mod api_req_mission;
mod api_req_nyukyo;
mod api_req_practice;
mod api_req_quest;
mod api_req_ranking;
mod api_req_sortie;
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
		.merge(Router::new().nest("/api_req_battle_midnight", api_req_battle_midnight::router()))
		.merge(Router::new().nest("/api_req_hensei", api_req_hensei::router()))
		.merge(Router::new().nest("/api_req_hokyu", api_req_hokyu::router()))
		.merge(Router::new().nest("/api_req_kaisou", api_req_kaisou::router()))
		.merge(Router::new().nest("/api_req_kousyou", api_req_kousyou::router()))
		.merge(Router::new().nest("/api_req_map", api_req_map::router()))
		.merge(Router::new().nest("/api_req_member", api_req_member::router()))
		.merge(Router::new().nest("/api_req_mission", api_req_mission::router()))
		.merge(Router::new().nest("/api_req_nyukyo", api_req_nyukyo::router()))
		.merge(Router::new().nest("/api_req_practice", api_req_practice::router()))
		.merge(Router::new().nest("/api_req_ranking", api_req_ranking::router()))
		.merge(Router::new().nest("/api_req_quest", api_req_quest::router()))
		.merge(Router::new().nest("/api_req_sortie", api_req_sortie::router()))
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
	let mock_path = format!("kcsapi{req_path}.json");
	if let Ok(mut f) = GetOption::new_api_mocking().get(&state.kache, &mock_path, NoVersion).await {
		info!("🤖 mocking response for {}", req_path);
		let mut raw = String::new();
		f.read_to_string(&mut raw).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

		// check if mock_data starts with 'svdata='
		return if raw.starts_with("svdata=") {
			Ok(Response::new(raw.into()))
		} else {
			// append 'svdata=' to the beginning of the response
			Ok(Response::new(format!("svdata={raw}").into()))
		};
	}

	Ok(next.run(Request::from_parts(parts, body)).await)
}

#[cfg(test)]
pub(super) mod test_utils {
	use super::*;
	use axum::Extension;
	use emukc_internal::prelude::*;
	use std::{path::PathBuf, sync::Arc};
	use tempfile::TempDir;

	use crate::{net::auth::GameSession, state::State};

	pub(super) struct TestContext {
		#[allow(dead_code)]
		pub cache_root: TempDir,
		pub state: Arc<State>,
		pub session: GameSession,
	}

	pub(super) async fn new_test_context() -> TestContext {
		let cache_root = tempfile::tempdir().unwrap();
		let db = Arc::new(new_mem_db().await.unwrap());
		let codex = Codex::load_without_cache_source(
			PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex"),
		)
		.unwrap();
		let cache_path = cache_root.path().join("cache");
		std::fs::create_dir_all(&cache_path).unwrap();
		let kache = Arc::new(
			Kache::builder()
				.with_cache_root(cache_path)
				.with_gadgets_cdn("https://example.invalid/gadgets".to_string())
				.with_content_cdn("https://example.invalid/content".to_string())
				.build()
				.unwrap(),
		);
		let state = Arc::new(State {
			db,
			kache,
			codex: Arc::new(codex),
		});

		let account = state.sign_up("router-test", "1234567").await.unwrap();
		let profile = state.new_profile(&account.access_token.token, "router-admin").await.unwrap();
		let session =
			state.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

		TestContext {
			cache_root,
			state,
			session: GameSession {
				token: session.session.token.clone(),
				profile: session.profile.clone(),
			},
		}
	}

	pub(super) async fn seed_single_ship_fleet(state: &Arc<State>, profile_id: i64) {
		let ship = state.add_ship(profile_id, 951).await.unwrap();
		state.update_fleet_ships(profile_id, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	}

	pub(super) fn app_state(state: &Arc<State>) -> AppState {
		Extension(state.clone())
	}
}
