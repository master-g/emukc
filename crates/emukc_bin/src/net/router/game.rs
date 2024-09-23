use axum::{
	extract::{Host, Path, Query},
	middleware,
	response::{Html, IntoResponse, Redirect, Response},
	routing::get,
	Extension, Router,
};
use serde::{Deserialize, Serialize};
use tera::Tera;

use crate::net::assets::GameSiteAssets;

pub(super) fn router() -> Router {
	Router::new()
		.route("/css/*path", get(css)) // css/*
		.route("/js/*path", get(js)) // js/*
		.merge(
			Router::new()
				.route("/", get(home)) // game home
				.route_layer(middleware::from_fn(auth_middleware))
				.route("/game/*path", get(game)), // game content
		)
}

// spkc/index.html
async fn home(Host(host): Host, Extension(user): Extension<AuthUser>) -> impl IntoResponse {
	// prepare html
	let html = GameSiteAssets::get("emukc/index.html").unwrap();
	let html = std::str::from_utf8(html.data.as_ref()).unwrap();

	// prepare parameters
	let parent = format!("//{}/netgame/social/", host);
	let parent = urlencoding::encode(&parent);
	let token = user.0.access_token().unwrap().token;

	let uid = user.0.uid();

	let mut tera = Tera::default();
	let mut context = tera::Context::new();
	context.insert("uid", &uid);
	context.insert("parent", &parent);
	context.insert("token", &token);
	let url = "/spkc/game/ifr.html?synd=dmm&container=dmm&owner={{uid}}&viewer={{uid}}&aid=854854&mid=29080258&country=jp&lang=ja&view=canvas&parent={{parent}}&access_token={{token}}&st={{token}}#rpctoken=1131055973";
	let url = tera.render_str(url, &context).unwrap();
	context.insert("ifr_url", &url);
	let result = tera.render_str(html, &context).unwrap();

	Html(result)
}

// spkc/css/*
async fn css(Path(path): Path<String>) -> impl IntoResponse {
	GameStaticFile(format!("spkc/css/{}", path))
}

// spkc/js/*
async fn js(Path(path): Path<String>) -> impl IntoResponse {
	GameStaticFile(format!("spkc/js/{}", path))
}

// spkc/game/js/hijack.js
async fn hijack_js(uid: i64) -> impl IntoResponse {
	let raw = GameSiteAssets::get("spkc/game/js/hijack.js").unwrap();
	let raw = std::str::from_utf8(raw.data.as_ref()).unwrap();

	let mut tera = Tera::default();
	let mut context = tera::Context::new();
	context.insert("version", PKG_VERSION.as_str());
	context.insert("uid", &uid);

	tera.render_str(raw, &context).unwrap()
}

#[derive(Serialize, Deserialize, Debug)]
struct ViewerQuery {
	viewer: Option<i64>,
}

// spkc/game/*
async fn game(
	Host(host): Host,
	Path(path): Path<String>,
	Query(query): Query<ViewerQuery>,
) -> Response {
	if path.ends_with("hijack.js") {
		let uid = query.viewer.unwrap_or(0);
		return hijack_js(uid).await.into_response();
	} else if path.ends_with("ifr.html") {
		let uid = query.viewer.unwrap_or(0);
		let raw = GameSiteAssets::get("spkc/game/ifr.html").unwrap();
		let raw = std::str::from_utf8(raw.data.as_ref()).unwrap();
		let mut tera = Tera::default();
		let mut context = tera::Context::new();
		context.insert("uid", &uid);
		let result = tera.render_str(raw, &context).unwrap();
		return Html(result).into_response();
	}

	let rel_path = format!("spkc/game/{}", path);
	if GameSiteAssets::get(&rel_path).is_some() {
		GameStaticFile(rel_path).into_response()
	} else {
		// not embedded, redirect to the real path
		Redirect::temporary(format!("//{}/gadgets/{}", host, path).as_str()).into_response()
	}
}
