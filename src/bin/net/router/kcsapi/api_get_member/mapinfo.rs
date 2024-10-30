use std::collections::BTreeMap;

use axum::Extension;
use emukc::{db::entity::profile::map_record, model::profile::map_record::DEFAULT_MAP_RECORDS};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Resp {
	api_air_base: Vec<KcApiAirBase>,
	api_air_base_expanded_info: Vec<KcApiAirBaseExpandedInfo>,
	api_map_info: Vec<KcApiMapInfo>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let airbases = state.get_airbases(pid).await?;
	let api_air_base_expanded_info = airbases
		.iter()
		.map(|v| KcApiAirBaseExpandedInfo {
			api_area_id: v.area_id,
			api_maintenance_level: v.maintenance_level,
		})
		.collect();
	let api_air_base = airbases.into_iter().map(std::convert::Into::into).collect();

	let map_records: BTreeMap<i64, map_record::Model> =
		state.get_map_records(pid).await?.into_iter().map(|r| (r.id, r)).collect();

	let api_map_info = DEFAULT_MAP_RECORDS
		.clone()
		.into_iter()
		.map(|mut info| {
			if let Some(record) = map_records.get(&info.id) {
				info.cleared = record.cleared;
				if let Some(ctx) = info.defeat_ctx.as_mut() {
					ctx.defeat_count = record.defeat_count.unwrap_or(0);
				}
			}

			info.into()
		})
		.collect();

	Ok(KcApiResponse::success(&Resp {
		api_air_base,
		api_air_base_expanded_info,
		api_map_info,
	}))
}
