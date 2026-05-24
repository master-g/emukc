use axum::{
    Extension, Router,
    extract::{Path, Query},
    http::Uri,
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use emukc_internal::prelude::PKG_VERSION;
use serde::{Deserialize, Serialize};
use tera::Tera;

use crate::net::{
    AppState,
    assets::{GameSiteAssets, GameStaticFile},
    auth::{GameSession, kcs_api_auth_middleware},
};
use crate::state::State;

pub(super) fn router() -> Router {
    Router::new()
        .route("/css/{*path}", get(css)) // css/*
        .route("/js/{*path}", get(js)) // js/*
        .route("/p/{*path}", get(p)) // p.dmm.com assets
        .route("/game/{*path}", get(game)) // game content (no auth — static assets)
        .merge(
            Router::new()
                .route("/", get(home)) // game home
                .route("/game/payment.html", get(payment_html)) // payment confirmation
                .route_layer(middleware::from_fn(kcs_api_auth_middleware)),
        )
}

// emukc/index.html
async fn home(uri: Uri, Extension(session): Extension<GameSession>) -> impl IntoResponse {
    // prepare html
    let html = GameSiteAssets::get("emukc/index.html").unwrap();
    let html = std::str::from_utf8(html.data.as_ref()).unwrap();

    // prepare parameters
    let host = uri.authority().map(http::uri::Authority::as_str).unwrap_or("localhost:8080");
    let parent = format!("//{host}/netgame/social/");
    let parent = urlencoding::encode(&parent);

    let token = session.token;
    let profile_id = session.profile.id;

    let mut tera = Tera::default();
    let mut context = tera::Context::new();
    context.insert("uid", &profile_id);
    context.insert("parent", &parent);
    context.insert("token", &token);
    let url = "/emukc/game/ifr.html?synd=dmm&container=dmm&owner={{uid}}&viewer={{uid}}&aid=854854&mid=29080258&country=jp&lang=ja&view=canvas&parent={{parent}}&st={{token}}#rpctoken=1131055973";
    let url = tera.render_str(url, &context).unwrap();
    context.insert("ifr_url", &url);
    let result = tera.render_str(html, &context).unwrap();

    Html(result)
}

// emukc/css/*
async fn css(Path(path): Path<String>) -> impl IntoResponse {
    GameStaticFile(format!("emukc/css/{path}"))
}

// emukc/js/*
async fn js(Path(path): Path<String>) -> impl IntoResponse {
    GameStaticFile(format!("emukc/js/{path}"))
}

// emukc/game/js/hijack.js
async fn hijack_js(uid: i64) -> impl IntoResponse {
    let raw = GameSiteAssets::get("emukc/game/js/hijack.js").unwrap();
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

#[derive(Serialize, Deserialize, Debug)]
struct PaymentHtmlQuery {
    payment_id: Option<String>,
    st: Option<String>,
}

// emukc/game/payment.html
async fn payment_html(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Query(query): Query<PaymentHtmlQuery>,
) -> Response {
    let state: &State = state.as_ref();

    let payment_id = match query.payment_id {
        Some(id) => id,
        None => return Html("missing payment_id".to_string()).into_response(),
    };

    let session_data = match state.payment_store.get(&payment_id) {
        Some(s) => s,
        None => return Html("payment session not found".to_string()).into_response(),
    };

    if session_data.profile_id != session.profile.id {
        return Html("access denied".to_string()).into_response();
    }

    let raw = GameSiteAssets::get("emukc/game/payment.html").unwrap();
    let raw = std::str::from_utf8(raw.data.as_ref()).unwrap();

    let mut tera = Tera::default();
    let mut context = tera::Context::new();
    context.insert("payment_id", &session_data.payment_id);
    context.insert("token", &session.token);
    context.insert("sku_id", &session_data.sku_id);
    context.insert("name", &session_data.name);
    context.insert("description", &session_data.description);
    context.insert("price", &session_data.price);
    context.insert("total_price", &(session_data.price * session_data.count));
    context.insert("it", &serde_json::json!({ "count": session_data.count }));

    match tera.render_str(raw, &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => Html(format!("template error: {e}")).into_response(),
    }
}

// emukc/p/* (p.dmm.com static assets)
async fn p(Path(path): Path<String>) -> impl IntoResponse {
    GameStaticFile(format!("emukc/p/{path}"))
}

// emukc/game/*
async fn game(uri: Uri, Path(path): Path<String>, Query(query): Query<ViewerQuery>) -> Response {
    let host = uri.authority().map(http::uri::Authority::as_str).unwrap_or("localhost:8080");

    if path.ends_with("hijack.js") {
        let uid = query.viewer.unwrap_or(0);
        return hijack_js(uid).await.into_response();
    } else if path.ends_with("ifr.html") {
        let uid = query.viewer.unwrap_or(0);
        let raw = GameSiteAssets::get("emukc/game/ifr.html").unwrap();
        let raw = std::str::from_utf8(raw.data.as_ref()).unwrap();
        let mut tera = Tera::default();
        let mut context = tera::Context::new();
        context.insert("uid", &uid);
        let result = tera.render_str(raw, &context).unwrap();
        return Html(result).into_response();
    }

    let rel_path = format!("emukc/game/{path}");
    if GameSiteAssets::get(&rel_path).is_some() {
        GameStaticFile(rel_path).into_response()
    } else {
        // not embedded, redirect to the real path
        Redirect::temporary(format!("//{host}/gadgets/{path}").as_str()).into_response()
    }
}
