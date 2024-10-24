use crate::{net::auth::GameSession, state::State};

use super::RpcParams;

pub(super) async fn exec(
	_state: &State,
	session: &GameSession,
	_params: RpcParams,
) -> serde_json::Value {
	serde_json::json!({
		"id": "viewer",
		"data": {
			"isOwner": true,
			"isViewer": true,
			"userType": "god", // "developer" | "staff" | etc
			"id": session.profile.id.to_string(),
			"thumbnailUrl": "https://pics.dmm.com/freegame/profile/m/male1/male1_mb.gif",
			"displayName": session.profile.name,
		}
	})
}
