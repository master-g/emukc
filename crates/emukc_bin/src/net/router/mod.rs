use axum::routing::Router;
use serde::{Deserialize, Serialize};

mod api;
mod gadget_html5;
mod gadgets;
mod game;
mod kcs;
mod kcs2;

#[derive(Serialize, Deserialize, Debug)]
struct KcVersionQuery {
	version: Option<String>,
}

pub(super) fn new() -> Router {
	Router::new()
		.merge(Router::new().nest("/api", api::router())) // api
		.merge(Router::new().nest("/gadget_html5", gadget_html5::router())) // gadget_html5
		.merge(Router::new().nest("/gadgets", gadgets::router())) // gadgets
		.merge(Router::new().nest("/kcs", kcs::router())) // gadget_html5
		.merge(Router::new().nest("/kcs2", kcs2::router())) // kcs2
		// .merge(Router::new().nest("/kcsapi", kcsapi::router())) // kcsapi
		// .merge(Router::new().nest("/netgame", netgame::router())) // netgame
		// .merge(Router::new().nest("/social", social::router())) // rpc
		.merge(Router::new().nest("/emukc", game::router())) // game site
}
