use axum::Extension;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::{model::kc2::KcApiIncentive, prelude::IncentiveOps};

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let incentives = state.0.confirm_incentives(session.profile.id).await?;
	Ok(KcApiResponse::success(&KcApiIncentive {
		api_count: incentives.len() as i64,
		api_item: if incentives.is_empty() {
			None
		} else {
			Some(incentives)
		},
	}))
}
