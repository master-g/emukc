use axum::response::IntoResponse;
use emukc_internal::{model::codex::CodexError, prelude::GameplayError};
pub use kcs::KcApiResponse;

use super::err::ApiError;

mod kcs;

#[derive(thiserror::Error, Debug)]
pub struct KcApiError(pub ApiError);

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

impl From<GameplayError> for KcApiError {
    fn from(value: GameplayError) -> Self {
        KcApiError(ApiError::from(value))
    }
}

impl From<CodexError> for KcApiError {
    fn from(value: CodexError) -> Self {
        KcApiError(ApiError::from(value))
    }
}

impl IntoResponse for KcApiError {
    fn into_response(self) -> axum::response::Response {
        KcApiResponse::failure(self.0.to_string().as_str()).into_response()
    }
}

pub type KcApiResult = std::result::Result<KcApiResponse, KcApiError>;

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use emukc_internal::db::sea_orm::DbErr;
    use http::StatusCode;

    use super::*;

    async fn read(err: KcApiError) -> (StatusCode, serde_json::Value) {
        let resp = err.into_response();
        let status = resp.status();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        let json = body.strip_prefix("svdata=").expect("svdata prefix");
        (status, serde_json::from_str(json).unwrap())
    }

    #[tokio::test]
    async fn database_error_keeps_the_kcsapi_contract_and_hides_the_message() {
        let (status, body) =
            read(KcApiError::from(GameplayError::Db(DbErr::Custom("secret-table".into())))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["api_result"], -1);
        let msg = body["api_result_msg"].as_str().unwrap();
        assert!(!msg.contains("secret-table"), "api_result_msg leaked the database message: {msg}");
    }

    #[tokio::test]
    async fn client_error_keeps_the_kcsapi_contract_and_the_message() {
        let (status, body) =
            read(KcApiError::from(GameplayError::Insufficient("fuel".into()))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["api_result"], -1);
        assert!(body["api_result_msg"].as_str().unwrap().contains("fuel"));
    }
}
