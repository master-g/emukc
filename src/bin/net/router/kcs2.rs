use axum::{
	Router,
	extract::{Path, Query},
	response::{Html, IntoResponse, Response},
	routing::get,
};
use emukc_internal::prelude::VERSION;
use http::{StatusCode, header};
use tera::Tera;

use crate::net::{
	AppState,
	assets::{self, GameSiteAssets, GameStaticFile},
	router::version::gen_version_png,
};

use super::KcVersionQuery;

pub(super) fn router() -> Router {
	Router::new().route("/{*path}", get(file_handler))
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

include!(concat!(env!("OUT_DIR"), "/git_version.rs"));

async fn file_handler(
	state: AppState,
	Path(path): Path<String>,
	Query(params): Query<KcVersionQuery>,
) -> Response {
	info!("kcs2: {}", path);

	// world info
	if path.contains("resources/world/") {
		let Some((w, h)) =
			path.strip_suffix(".png").and_then(|s| s.chars().last()).map(|c| match c {
				's' => (114, 19),
				't' | 'l' => (157, 27),
				_ => (0, 0),
			})
		else {
			return (StatusCode::BAD_REQUEST, "invalid version png").into_response();
		};

		if w == 0 || h == 0 {
			return (StatusCode::BAD_REQUEST, "invalid version png").into_response();
		}

		let ver = format!("EmuKC {}-{}", VERSION, GIT_HASH.to_uppercase());
		let Some(png) = gen_version_png(&ver, w, h) else {
			return (StatusCode::INTERNAL_SERVER_ERROR, "failed to generate version png")
				.into_response();
		};

		return ([(header::CONTENT_TYPE, "image/png")], png).into_response();
	}

	// embedded
	let embed_path = format!("emukc/{path}");
	if GameSiteAssets::get(&embed_path).is_some() {
		return GameStaticFile(&embed_path).into_response();
	}

	// cache
	let cache_rel_path = format!("kcs2/{path}");

	if cache_rel_path.contains("index.php") {
		let Some(version) = params.version.as_deref() else {
			return (StatusCode::BAD_REQUEST, "version is required").into_response();
		};

		return index(version).await;
	}
	// } else if cache_rel_path.starts_with("kcs2/resources/world/") {
	// 	let filename = cache_rel_path.rsplit('/').next().unwrap();
	// 	let local_path = PathBuf::from("emukc/game/resources/world/").join(filename);
	// 	return GameStaticFile(local_path.to_str().unwrap().to_string()).into_response();
	// }

	assets::cache::get_file(state, &cache_rel_path, params.version.as_deref()).await.into_response()
}
