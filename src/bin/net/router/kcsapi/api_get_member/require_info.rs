use std::collections::BTreeMap;

use axum::Extension;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct UserBasic {
	api_member_id: i64,
	api_firstflag: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_basic: UserBasic,
	api_extra_supply: [i64; 2],
	api_furniture: Vec<KcApiFurniture>,
	api_kdock: Vec<KcApiKDock>,
	api_oss_setting: KcApiOssSetting,
	api_position_id: i64,
	api_skin_id: i64,
	api_slot_item: Vec<KcApiSlotItem>,
	api_unsetslot: BTreeMap<String, Vec<i64>>,
	api_useitem: Vec<KcApiUserItem>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let resp = build_require_info_response(state.0.as_ref(), pid).await?;
	Ok(KcApiResponse::success(&resp))
}

async fn build_require_info_response<T: GameOps + HasContext + ?Sized>(
	state: &T,
	pid: i64,
) -> Result<Resp, GameplayError> {
	let codex = state.codex();
	let (_, api_basic) = state.get_user_basic(pid).await?;
	let api_furniture = state.get_furnitures(pid).await?;
	let api_kdock = state.get_kdocks(pid).await?;
	let api_kdock = api_kdock.iter().map(|k| k.to_owned().into()).collect();

	let api_oss_setting = state.get_oss_settings(pid).await?;
	let api_game_settings = state.get_game_settings(pid).await?;
	let api_option_settings = state.get_option_settings(pid).await?;

	let api_extra_supply = api_basic.api_extra_supply;
	let api_position_id = api_game_settings.api_position_id;
	let api_skin_id = api_option_settings.map(|s| s.api_skin_id).unwrap_or(101);

	let api_slot_item = state.get_slot_items(pid).await?;
	let api_useitem = state.get_use_items(pid).await?;

	let unused_slot_items = state.get_unset_slot_items(pid).await?;
	let api_unsetslot = codex.convert_unused_slot_items_to_api(&unused_slot_items)?;

	Ok(Resp {
		api_basic: UserBasic {
			api_member_id: api_basic.api_member_id,
			api_firstflag: api_basic.api_firstflag,
		},
		api_extra_supply,
		api_furniture,
		api_kdock,
		api_oss_setting,
		api_position_id,
		api_skin_id,
		api_slot_item,
		api_unsetslot,
		api_useitem,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;
	use emukc_internal::db::prelude::new_mem_db;

	async fn new_game_session() -> ((emukc_internal::db::sea_orm::DbConn, Codex), StartGameInfo) {
		let db = new_mem_db().await.unwrap();
		let codex_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex");
		let codex = Codex::load_without_cache_source(codex_root).unwrap();
		let context = (db, codex);

		let account = context.sign_up("test", "1234567").await.unwrap();
		let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
		let session =
			context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

		(context, session)
	}

	#[tokio::test]
	async fn create_slotitem_is_visible_in_require_info() {
		let (context, session) = new_game_session().await;
		let pid = session.profile.id;

		let before = build_require_info_response(&context, pid).await.unwrap();
		let craftable = context
			.1
			.slotitem_extra_info
			.values()
			.find(|item| item.craftable)
			.unwrap()
			.api_id;
		let costs = vec![
			(MaterialCategory::Fuel, 10),
			(MaterialCategory::Ammo, 10),
			(MaterialCategory::Steel, 10),
			(MaterialCategory::Bauxite, 10),
			(MaterialCategory::DevMat, 1),
		];

		let (ids, _materials) = context.create_slotitem(pid, &[craftable], &costs).await.unwrap();
		let created_id = ids[0];
		assert!(created_id > 0);

		let after = build_require_info_response(&context, pid).await.unwrap();
		assert_eq!(after.api_slot_item.len(), before.api_slot_item.len() + 1);
		assert!(after.api_slot_item.iter().any(|item| item.api_id == created_id));

		let type3 = context
			.1
			.find::<ApiMstSlotitem>(&craftable)
			.unwrap()
			.api_type[2]
			.to_string();
		let unset_key = format!("api_slottype{type3}");
		assert!(
			after
				.api_unsetslot
				.get(&unset_key)
				.is_some_and(|items| items.contains(&created_id))
		);
	}
}
