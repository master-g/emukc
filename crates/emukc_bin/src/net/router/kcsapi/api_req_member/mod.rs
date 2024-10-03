mod get_incentive;
// mod get_practice_enemyinfo;

use axum::{routing::post, Router};

pub(super) fn router() -> Router {
	Router::new().route("/get_incentive", post(get_incentive::handler))
	// .route("/get_practice_enemyinfo", post(get_practice_enemyinfo::handler))
}
