use axum::response::{IntoResponse, Response};
use http::{header, StatusCode};

pub(super) mod cache;

// FIXME: naming here is somehow confusing

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
#[folder = "$CARGO_MANIFEST_DIR/assets/www/gadgets"]
pub(super) struct GameGadgetsAssets;

#[allow(dead_code)]
pub(super) struct GameGadgetsFile<T>(pub T);

impl<T> IntoResponse for GameGadgetsFile<T>
where
	T: Into<String>,
{
	fn into_response(self) -> Response {
		let path = self.0.into();

		match GameGadgetsAssets::get(path.as_str()) {
			Some(content) => {
				let mime = mime_guess::from_path(path).first_or_octet_stream();
				([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
			}
			None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
		}
	}
}