use axum::{
	extract::{Path, Query},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};

use crate::net::{
	assets::{self},
	AppState,
};

use super::KcVersionQuery;

pub(super) fn router() -> Router {
	Router::new().route("/{*path}", get(file_handler))
}

async fn file_handler(
	state: AppState,
	Path(path): Path<String>,
	Query(params): Query<KcVersionQuery>,
) -> Response {
	let rel_path = format!("gadget_html5/{}", path);
	assets::cache::get_file(state, &rel_path, params.version.as_deref()).await.into_response()
}
