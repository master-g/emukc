use std::path::PathBuf;

use axum::{
	extract::{Path, Query},
	response::{Html, IntoResponse, Response},
	routing::get,
	Router,
};
use http::StatusCode;
use tera::Tera;

use crate::net::{
	assets::{self, GameSiteAssets, GameStaticFile},
	AppState,
};

use super::KcVersionQuery;

pub(super) fn router() -> Router {
	Router::new().route("/*path", get(file_handler))
}

async fn index(version: &str) -> Response {
	let html = GameSiteAssets::get("emukc/game/index.php").unwrap();
	let html = std::str::from_utf8(html.data.as_ref()).unwrap();

	let mut context = tera::Context::new();
	context.insert("version", &version);

	let mut tera = Tera::default();

	let result = tera.render_str(html, &context).unwrap();

	Html(result).into_response()
}

async fn file_handler(
	state: AppState,
	Path(path): Path<String>,
	Query(params): Query<KcVersionQuery>,
) -> Response {
	let real_path = PathBuf::from("kcs2").join(path).to_string_lossy().to_string();

	if real_path.contains("index.php") {
		let version = if let Some(version) = params.version.as_deref() {
			version
		} else {
			return (StatusCode::BAD_REQUEST, "version is required").into_response();
		};
		return index(version).await;
	} else if real_path.starts_with("kcs2/resources/world/") {
		let filename = real_path.rsplit('/').next().unwrap();
		let local_path = PathBuf::from("emukc/game/resources/world/").join(filename);
		return GameStaticFile(local_path.to_str().unwrap().to_string()).into_response();
	}

	assets::cache::get_file(state, &real_path, params.version.as_deref()).await.into_response()
}
