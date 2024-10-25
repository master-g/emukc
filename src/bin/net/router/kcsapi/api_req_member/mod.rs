use axum::{routing::post, Router};

mod get_incentive;
mod set_oss_condition;
mod update_tutorial_progress;
mod updatecomment;
// mod get_practice_enemyinfo;

pub(super) fn router() -> Router {
	Router::new()
		.route("/get_incentive", post(get_incentive::handler))
		.route("/set_oss_condition", post(set_oss_condition::handler))
		.route("/update_tutorial_progress", post(update_tutorial_progress::handler))
		.route("/updatecomment", post(updatecomment::handler))
	// .route("/get_practice_enemyinfo", post(get_practice_enemyinfo::handler))
}
