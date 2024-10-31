use emukc_internal::time::{chrono, KcTime};

use crate::{net::auth::GameSession, state::State};

use super::RpcParams;

pub(super) async fn exec(
	_state: &State,
	session: &GameSession,
	_params: RpcParams,
) -> serde_json::Value {
	let now = chrono::Utc::now().timestamp_millis();

	let now = KcTime::format_date(now, "T");
	let uid = session.profile.id.to_string();
	let resp = serde_json::json!({
		"id": "key",
		"data":[{
			"textId": uid,
			"appId": "854854",
			"authorId": uid,
			"ownerId": uid,
			"ctime": now,
			"mtime": now,
			"status": 0,
		}]
	});

	serde_json::json!(resp)
}
