use axum::{middleware, Router};

use crate::net::auth;

mod ship;

pub(super) fn router() -> Router {
	Router::new()
		.merge(Router::new().nest("/ship", ship::router()))
		.route_layer(middleware::from_fn(auth::auth_middleware))
}
