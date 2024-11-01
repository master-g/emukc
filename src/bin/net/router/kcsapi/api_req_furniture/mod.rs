use axum::{routing::post, Router};

mod buy;
mod change;
mod music_list;
mod music_play;
mod radio_play;
mod set_portbgm;

pub(super) fn router() -> Router {
	Router::new()
		.route("/buy", post(buy::handler))
		.route("/change", post(change::handler))
		.route("/music_list", post(music_list::handler))
		.route("/music_play", post(music_play::handler))
		.route("/radio_play", post(radio_play::handler))
		.route("/set_portbgm", post(set_portbgm::handler))
}
