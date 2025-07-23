use axum::{
	Router,
	extract::Path,
	response::{IntoResponse, Response},
	routing::get,
};

use crate::net::{AppState, assets};

pub(super) fn router() -> Router {
	Router::new().route("/{*path}", get(file_handler))
}

async fn file_handler(state: AppState, Path(path): Path<String>) -> Response {
	info!("html: {}", path);

	let cache_rel_path = format!("html/{path}");

	assets::cache::get_file(state, &cache_rel_path, None).await.into_response()
}
