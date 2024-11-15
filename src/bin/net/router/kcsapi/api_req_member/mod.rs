use axum::{routing::post, Router};

mod get_event_selected_reward;
mod get_incentive;
mod get_practice_enemyinfo;
mod itemuse;
mod itemuse_cond;
mod payitemuse;
mod set_flagship_position;
mod set_friendly_request;
mod set_option_setting;
mod set_oss_condition;
mod update_tutorial_progress;
mod updatecomment;
mod updatedeckname;

pub(super) fn router() -> Router {
	Router::new()
		.route("/get_event_selected_reward", post(get_event_selected_reward::handler))
		.route("/get_incentive", post(get_incentive::handler))
		.route("/get_practice_enemyinfo", post(get_practice_enemyinfo::handler))
		.route("/itemuse", post(itemuse::handler))
		.route("/itemuse_cond", post(itemuse_cond::handler))
		.route("/payitemuse", post(payitemuse::handler))
		.route("/set_flagship_position", post(set_flagship_position::handler))
		.route("/set_friendly_request", post(set_friendly_request::handler))
		.route("/set_option_setting", post(set_option_setting::handler))
		.route("/set_oss_condition", post(set_oss_condition::handler))
		.route("/update_tutorial_progress", post(update_tutorial_progress::handler))
		.route("/updatecomment", post(updatecomment::handler))
		.route("/updatedeckname", post(updatedeckname::handler))
}
