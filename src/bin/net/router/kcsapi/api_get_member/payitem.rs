use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Item {
	api_payitem_id: i64,
	api_type: i64,
	api_name: String,
	api_description: String,
	api_price: i64,
	api_count: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let pay_items = state.get_pay_items(pid).await?;

	let resp: Vec<Item> = pay_items
		.into_iter()
		.filter_map(|v| {
			let Ok(mst) = state.codex().find::<ApiMstPayitem>(&v.api_id) else {
				return None;
			};

			Some(Item {
				api_payitem_id: v.api_id,
				api_type: mst.api_type,
				api_name: mst.api_name.clone(),
				api_description: mst.api_description.clone(),
				api_price: mst.api_price,
				api_count: v.api_count,
			})
		})
		.collect();

	Ok(KcApiResponse::success(&resp))
}
