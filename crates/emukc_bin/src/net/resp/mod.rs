use axum::response::IntoResponse;
use kcs::KcApiResponse;

mod kcs;

// TODO: add a error.rs to net module for common error handling

#[derive(thiserror::Error, Debug)]
pub struct KcApiError(Error);

impl std::fmt::Display for KcApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<Error> for KcApiError {
	fn from(e: Error) -> Self {
		KcApiError(e)
	}
}

impl IntoResponse for KcApiError {
	fn into_response(self) -> axum::response::Response {
		KcApiResponse::failure(self.0.to_string().as_str()).into_response()
	}
}

pub type KcApiResult = std::result::Result<KcApiResponse, KcApiError>;
