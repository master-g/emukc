mod get_incentive;
mod update_tutorial_progress;
mod updatecomment;
// mod get_practice_enemyinfo;

use axum::{routing::post, Router};

pub(super) fn router() -> Router {
	Router::new()
		.route("/get_incentive", post(get_incentive::handler))
		.route("/update_tutorial_progress", post(update_tutorial_progress::handler))
		.route("/updatecomment", post(updatecomment::handler))
	// .route("/get_practice_enemyinfo", post(get_practice_enemyinfo::handler))
}
