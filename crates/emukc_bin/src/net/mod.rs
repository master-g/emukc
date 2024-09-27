use std::net::SocketAddr;

use anyhow::Result;
use axum::{http, Extension};
use axum_server::Handle;
use signal::graceful_shutdown;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::{
	add_extension::AddExtensionLayer,
	compression::CompressionLayer,
	cors::{Any, CorsLayer},
	request_id::MakeRequestUuid,
	trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
	ServiceBuilderExt,
};

use crate::{
	cfg::AppConfig,
	state::{State, StateArc},
};

mod assets;
mod auth;
mod err;
mod header;
mod resp;
mod router;
mod signal;

const LOG_TAG: &str = "emukc::net";

type AppState = Extension<StateArc>;

/// Start the network service.
///
/// This function will start the network service and listen for incoming connections.
///
/// # Arguments
///
/// * `ct` - The cancellation token to stop the network service.
/// * `cfg` - The application configuration.
/// * `state` - The application state.
pub(super) async fn run(ct: CancellationToken, cfg: &AppConfig, state: &State) -> Result<()> {
	// axum service
	let service = ServiceBuilder::new()
		.catch_panic()
		.set_x_request_id(MakeRequestUuid)
		.propagate_x_request_id();

	let state = StateArc::new(state.clone());

	let service = service
		.layer(CompressionLayer::new().br(true).deflate(true).gzip(true).zstd(true))
		.layer(AddExtensionLayer::new(state))
		.layer(
			TraceLayer::new_for_http()
				.make_span_with(DefaultMakeSpan::new().include_headers(false))
				.on_response(DefaultOnResponse::new().include_headers(false)),
		)
		.layer(header::add_version_header())
		.layer(
			CorsLayer::new()
				.allow_origin(Any)
				.allow_methods([
					http::Method::GET,
					http::Method::PUT,
					http::Method::POST,
					http::Method::DELETE,
					http::Method::PATCH,
					http::Method::OPTIONS,
				])
				.allow_headers(Any)
				.max_age(std::time::Duration::from_secs(86400)),
		); // CORS;

	// axum app
	let axum_app = router::new().layer(service);

	// graceful shutdown
	let handle = Handle::new();
	let shutdown_handler = graceful_shutdown(ct, handle.clone());

	info!(target: LOG_TAG, "listening on: {}", &cfg.bind);

	axum_server::bind(cfg.bind)
		.handle(handle)
		.serve(axum_app.into_make_service_with_connect_info::<SocketAddr>())
		.await?;

	// wait for the shutdown signal
	let _ = shutdown_handler.await;

	info!(target: LOG_TAG, "server stopped. Goodbye!");

	Ok(())
}
