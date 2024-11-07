use axum::Extension;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let incentives = state.confirm_incentives(pid).await?;

	// incentives.push(KcApiIncentiveItem {
	// 	api_mode: 1,
	// 	api_type: 1,
	// 	api_mst_id: 1699,
	// 	api_getmes: None,
	// 	api_slotitem_level: None,
	// 	amount: 0,
	// 	alv: 0,
	// });
	// incentives.push(KcApiIncentiveItem {
	// 	api_mode: 1,
	// 	api_type: 1,
	// 	api_mst_id: 1581,
	// 	api_getmes: None,
	// 	api_slotitem_level: None,
	// 	amount: 0,
	// 	alv: 0,
	// });

	Ok(KcApiResponse::success(&KcApiIncentive {
		api_count: incentives.len() as i64,
		api_item: if incentives.is_empty() {
			None
		} else {
			Some(incentives)
		},
	}))
}
