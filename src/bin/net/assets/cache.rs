use axum::{
	body::Body,
	response::{IntoResponse, Response},
};
use http::StatusCode;
use tokio_util::io::ReaderStream;

use crate::net::AppState;

/// Cache file handler
///
/// # Arguments
///
/// - `app` - the application state
/// - `rel_path` - the relative path of the file
/// - `version` - the version of the file
pub async fn get_file(app: AppState, rel_path: &str, version: Option<&str>) -> impl IntoResponse {
	if rel_path.ends_with(".min.map") || rel_path.ends_with(".js.map") {
		// we don't want to serve source maps
		return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
	}

	let Ok(f) = app.kache.get(rel_path, version).await else {
		error!("❗️ cannot get file: {}", rel_path);
		return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
	};
	let stream = ReaderStream::new(f);

	Response::builder().status(StatusCode::OK).body(Body::from_stream(stream)).unwrap()
}
