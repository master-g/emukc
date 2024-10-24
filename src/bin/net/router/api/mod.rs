use axum::Router;

mod v1;

pub(super) fn router() -> Router {
	Router::new().merge(Router::new().nest("/v1", v1::router()))
}
