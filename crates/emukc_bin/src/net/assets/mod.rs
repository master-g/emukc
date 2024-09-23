use axum::response::{IntoResponse, Response};
use http::{header, StatusCode};

pub(super) mod cache;

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/assets/www"]
pub(super) struct GameSiteAssets;

pub(super) struct GameStaticFile<T>(pub T);

impl<T> IntoResponse for GameStaticFile<T>
where
	T: Into<String>,
{
	fn into_response(self) -> Response {
		let path = self.0.into();

		match GameSiteAssets::get(path.as_str()) {
			Some(content) => {
				let mime = mime_guess::from_path(path).first_or_octet_stream();
				([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
			}
			None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
		}
	}
}

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/assets/html"]
pub(super) struct GameHtmlAssets;

#[allow(dead_code)]
pub(super) struct GameHtmlFile<T>(pub T);

impl<T> IntoResponse for GameHtmlFile<T>
where
	T: Into<String>,
{
	fn into_response(self) -> Response {
		let path = self.0.into();

		match GameHtmlAssets::get(path.as_str()) {
			Some(content) => {
				let mime = mime_guess::from_path(path).first_or_octet_stream();
				([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
			}
			None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
		}
	}
}
