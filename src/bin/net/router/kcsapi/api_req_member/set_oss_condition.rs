use std::collections::HashMap;

use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Debug)]
pub(super) struct Params {
	api_language_type: i64,

	api_oss_items: [i64; 8],
}

impl<'de> Deserialize<'de> for Params {
	fn deserialize<D>(deserializer: D) -> Result<Params, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let map: HashMap<String, String> = Deserialize::deserialize(deserializer)?;

		let api_language_type = map
			.get("api_language_type")
			.ok_or_else(|| serde::de::Error::missing_field("api_language_type"))?
			.parse::<i64>()
			.map_err(serde::de::Error::custom)?;

		let mut oss_items = [0i64; 8];
		for (i, item) in oss_items.iter_mut().enumerate() {
			let key = format!("api_oss_items[{i}]");
			let value = map
				.get(&key)
				.ok_or_else(|| serde::de::Error::missing_field("api_oss_items"))?
				.parse::<i64>()
				.map_err(serde::de::Error::custom)?;
			*item = value;
		}

		Ok(Params {
			api_language_type,
			api_oss_items: oss_items,
		})
	}
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state.update_oss_settings(pid, params.api_language_type, &params.api_oss_items).await?;

	Ok(KcApiResponse::empty())
}
