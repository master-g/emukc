use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct VolumeSetting {
	/// BGM volume, 0-100
	pub api_vol_bgm: i64,
	/// Sound effect volume, 0-100
	pub api_vol_se: i64,
	/// Voice volume, 0-100
	pub api_vol_voice: i64,
	/// Secretary idle voice enabled
	pub api_v_be_left: i64,
	/// Mission completed voice enabled
	pub api_v_duty: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_skin_id: i64,
	api_volume_setting: Option<VolumeSetting>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;

	let option = state.get_option_settings(pid).await?;

	Ok(KcApiResponse::success(&Resp {
		api_skin_id: option.as_ref().map(|s| s.api_skin_id).unwrap_or(101),
		api_volume_setting: option.map(|s| VolumeSetting {
			api_vol_bgm: s.api_vol_bgm,
			api_vol_se: s.api_vol_se,
			api_vol_voice: s.api_vol_voice,
			api_v_be_left: s.api_v_be_left,
			api_v_duty: s.api_v_duty,
		}),
	}))
}
