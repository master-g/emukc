use axum::response::IntoResponse;
use kcs::KcApiResponse;

use super::err::ApiError;

mod kcs;

#[derive(thiserror::Error, Debug)]
pub struct KcApiError(ApiError);

impl std::fmt::Display for KcApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<ApiError> for KcApiError {
	fn from(e: ApiError) -> Self {
		KcApiError(e)
	}
}

impl IntoResponse for KcApiError {
	fn into_response(self) -> axum::response::Response {
		KcApiResponse::failure(self.0.to_string().as_str()).into_response()
	}
}

#[allow(unused)]
pub type KcApiResult = std::result::Result<KcApiResponse, KcApiError>;
