use axum::response::IntoResponse;

pub(super) async fn handler() -> impl IntoResponse {
	"hello world!"
}
