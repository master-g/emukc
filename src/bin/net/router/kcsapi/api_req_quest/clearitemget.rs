use std::collections::BTreeMap;

use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_quest_id: i64,
	#[serde(flatten, deserialize_with = "deserialize_select_idx_vec")]
	select_no: Option<Vec<i64>>,
}

fn deserialize_select_idx_vec<'de, D>(deserializer: D) -> Result<Option<Vec<i64>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::de::Error;
	use std::collections::HashMap;

	let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
	let mut select_map = BTreeMap::new();

	// collect all `api_select_no` fields
	for (key, value) in map {
		if key.starts_with("api_select_no") {
			let parsed_value = value
				.parse::<i64>()
				.map_err(|_| D::Error::custom(format!("Invalid number in {}: {}", key, value)))?;

			let index = if key == "api_select_no" {
				1
			} else if let Some(suffix) = key.strip_prefix("api_select_no") {
				suffix
					.parse::<usize>()
					.map_err(|_| D::Error::custom(format!("Invalid key format: {}", key)))?
			} else {
				return Err(D::Error::custom(format!("Unexpected key: {}", key)));
			};

			if index > 0 {
				select_map.insert(index, parsed_value);
			}
		}
	}

	// convert to Option<Vec<i64>>，keep the order by index
	if select_map.is_empty() {
		Ok(None)
	} else {
		// create Vec from BTreeMap values
		let values: Vec<i64> = select_map.into_values().collect();
		Ok(Some(values))
	}
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let resp =
		state.quest_clear_and_claim_reward(pid, params.api_quest_id, params.select_no).await?;

	Ok(KcApiResponse::success(&resp))
}
