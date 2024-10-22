use axum::Router;

mod auth;
mod debug;

pub(super) fn router() -> Router {
	Router::new()
		.merge(Router::new().nest("/auth", auth::router()))
		.merge(Router::new().nest("/debug", debug::router()))
}
