mod auth;

use axum::Router;

pub(super) fn router() -> Router {
	Router::new().merge(Router::new().nest("/auth", auth::router()))
}
