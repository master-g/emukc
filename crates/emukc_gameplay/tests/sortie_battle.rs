//! Sortie battle integration tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};

use emukc_bootstrap::prelude::build_final_map_catalog;
use emukc_db::{
	entity::profile::{self, map_record, quest, ship},
	prelude::new_mem_db,
	sea_orm::{
		ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
	},
};
use emukc_gameplay::prelude::*;
use emukc_model::{
	codex::{
		Codex,
		map::{MapDefinition, MapResetPolicy, MapVariantDefinition},
	},
	kc2::start2::{ApiMstMapinfo, ApiMstShip},
	thirdparty::{Kc3rdEnemyShip, Kc3rdEnemyShipSlotInfo, Kc3rdQuestRequirement},
};
use emukc_time::chrono::{TimeZone, Utc};

static PROFILE_ID_BUMP: AtomicI64 = AtomicI64::new(0);

async fn mock_context() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	(db, codex)
}

async fn mock_context_with_maps() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.data/temp");
	codex.maps = build_final_map_catalog(&data_root, &codex.manifest, None).unwrap();
	(db, codex)
}

async fn mock_context_with_repo_wikiwiki_maps() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	let asset_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../crates/emukc_bootstrap/assets/wikiwiki_map_catalog.json");
	let raw = std::fs::read_to_string(asset_path).unwrap();
	let wikiwiki_catalog = serde_json::from_str(&raw).unwrap();
	let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.data/temp");
	codex.maps =
		build_final_map_catalog(&data_root, &codex.manifest, Some(wikiwiki_catalog)).unwrap();
	ensure_enemy_manifest_entries(&mut codex);
	(db, codex)
}

fn ensure_enemy_manifest_entries(codex: &mut Codex) {
	let enemy_ids = codex
		.maps
		.maps
		.values()
		.flat_map(|definition| definition.variants.values())
		.flat_map(|variant| variant.enemy_fleets.values())
		.flat_map(|fleet| fleet.compositions.iter())
		.flat_map(|composition| composition.ship_ids.iter().copied())
		.collect::<std::collections::BTreeSet<_>>();

	for ship_id in enemy_ids {
		if codex.manifest.find_ship(ship_id).is_none() {
			codex.manifest.api_mst_ship.push(ApiMstShip {
				api_id: ship_id,
				api_name: format!("enemy-{ship_id}"),
				api_stype: 2,
				api_soku: 0,
				api_slot_num: 0,
				api_taik: Some([1, 1]),
				api_houg: Some([0, 0]),
				api_raig: Some([0, 0]),
				api_tyku: Some([0, 0]),
				api_souk: Some([0, 0]),
				api_maxeq: Some([0; 5]),
				api_fuel_max: Some(0),
				api_bull_max: Some(0),
				..ApiMstShip::default()
			});
		}
		if let Some(mst) =
			codex.manifest.api_mst_ship.iter_mut().find(|ship| ship.api_id == ship_id)
		{
			mst.api_taik.get_or_insert([1, 1]);
			mst.api_houg.get_or_insert([0, 0]);
			mst.api_raig.get_or_insert([0, 0]);
			mst.api_tyku.get_or_insert([0, 0]);
			mst.api_souk.get_or_insert([0, 0]);
			mst.api_maxeq.get_or_insert([0; 5]);
			mst.api_fuel_max.get_or_insert(0);
			mst.api_bull_max.get_or_insert(0);
		}
		codex.enemy_ship_extra.entry(ship_id).or_insert(Kc3rdEnemyShip {
			api_id: ship_id,
			name: format!("enemy-{ship_id}"),
			yomi: format!("enemy-{ship_id}"),
			stype: codex.manifest.find_ship(ship_id).map(|mst| mst.api_stype).unwrap_or(2),
			ctype: codex.manifest.find_ship(ship_id).map(|mst| mst.api_ctype).unwrap_or(0),
			hp: codex.manifest.find_ship(ship_id).and_then(|mst| mst.api_taik).unwrap_or([1, 1])[0],
			firepower: codex
				.manifest
				.find_ship(ship_id)
				.and_then(|mst| mst.api_houg)
				.unwrap_or([0, 0])[0],
			torpedo: codex
				.manifest
				.find_ship(ship_id)
				.and_then(|mst| mst.api_raig)
				.unwrap_or([0, 0])[0],
			aa: codex.manifest.find_ship(ship_id).and_then(|mst| mst.api_tyku).unwrap_or([0, 0])[0],
			armor: codex.manifest.find_ship(ship_id).and_then(|mst| mst.api_souk).unwrap_or([0, 0])
				[0],
			evasion: 0,
			asw: 0,
			los: 0,
			luck: 0,
			speed: codex.manifest.find_ship(ship_id).map(|mst| mst.api_soku).unwrap_or(0),
			range: codex.manifest.find_ship(ship_id).and_then(|mst| mst.api_leng).unwrap_or(0),
			rarity: 0,
			backs: codex.manifest.find_ship(ship_id).and_then(|mst| mst.api_backs).unwrap_or(0),
			slot_num: codex.manifest.find_ship(ship_id).map(|mst| mst.api_slot_num).unwrap_or(0),
			maxeq: codex
				.manifest
				.find_ship(ship_id)
				.and_then(|mst| mst.api_maxeq)
				.unwrap_or([0; 5]),
			slots: vec![Kc3rdEnemyShipSlotInfo {
				item_id: 525,
				onslot: 0,
			}],
		});
	}
}

async fn new_game_session() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

async fn new_game_session_with_maps() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context_with_maps().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-maps-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

async fn new_game_session_with_repo_wikiwiki_maps()
-> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context_with_repo_wikiwiki_maps().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-repo-maps-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

async fn ensure_started_quest(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) {
	if !context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.iter()
		.any(|record| record.quest_id == quest_id)
	{
		let quest_manifest = context.1.quest.get(&quest_id).unwrap();
		let (requirements, requirement_type) = match &quest_manifest.requirements {
			Kc3rdQuestRequirement::And(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::And)
			}
			Kc3rdQuestRequirement::OneOf(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::OneOf)
			}
			Kc3rdQuestRequirement::Sequential(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::Sequential)
			}
		};

		quest::progress::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			quest_id: ActiveValue::Set(quest_id),
			status: ActiveValue::Set(quest::progress::Status::Idle),
			progress: ActiveValue::Set(quest::progress::Progress::Empty),
			period: ActiveValue::Set(quest_manifest.period.into()),
			start_since: ActiveValue::Set(Utc::now()),
			requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
			requirement_type: ActiveValue::Set(requirement_type),
		}
		.insert(&context.0)
		.await
		.unwrap();
	}
	context.quest_start(profile_id, quest_id).await.unwrap();
}

async fn quest_progress_of(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) -> quest::progress::Progress {
	context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.into_iter()
		.find(|record| record.quest_id == quest_id)
		.unwrap()
		.progress
}

fn path_to_boss(codex: &Codex, map_id: i64) -> Vec<i64> {
	let definition = codex.maps.map_definition(map_id).unwrap();
	let variant = definition.variant("").unwrap();
	let start = variant.first_progress_cell_no().unwrap();
	let boss = variant.boss_cell_no;

	fn dfs(
		variant: &emukc_model::codex::map::MapVariantDefinition,
		current: i64,
		target: i64,
		path: &mut Vec<i64>,
	) -> bool {
		path.push(current);
		if current == target {
			return true;
		}

		let Some(cell) = variant.cell(current) else {
			path.pop();
			return false;
		};
		for next in &cell.next_cells {
			if dfs(variant, *next, target, path) {
				return true;
			}
		}

		path.pop();
		false
	}

	let mut path = Vec::new();
	assert!(dfs(variant, start, boss, &mut path));
	path
}

#[tokio::test]
async fn sortie_start_battle_result_flow_updates_stats() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	let before_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let before_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();

	let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	assert_eq!(start.maparea_id, 1);
	assert_eq!(start.mapinfo_no, 1);
	assert!(start.boss_cell_no >= 1);

	let battle = context.sortie_battle(pid, 1).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert!(!battle.api_ship_ke.is_empty());

	let result = context.sortie_battle_result(pid).await.unwrap();
	assert!(!result.api_win_rank.is_empty());
	assert_eq!(result.api_quest_name, "鎮守府正面海域");
	assert_eq!(result.api_quest_level, 1);

	let (profile, _) = context.get_user_basic(pid).await.unwrap();
	assert_eq!(profile.sortie_wins + profile.sortie_loses, 1);
	let after_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let after_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();
	assert_eq!(after_profile.experience, before_profile.experience + result.api_get_exp);
	assert_eq!(after_ship.exp_now, before_ship.exp_now + result.api_get_ship_exp[1]);
}

#[tokio::test]
async fn loaded_map_catalog_supports_start_and_next_flow() {
	let (context, session) = new_game_session_with_repo_wikiwiki_maps().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	assert_eq!(start.maparea_id, 1);
	assert_eq!(start.mapinfo_no, 1);
	assert!(start.cell_data.len() >= 4);
	assert_eq!(start.cell_no, 1);
	assert_eq!(start.cell_data[0].master_cell_id, 3001);
	assert_eq!(start.cell_data[1].master_cell_id, 3002);
	assert!(start.cell_data[1].passed);
	assert!(start.bosscomp);
	assert_eq!(start.airsearch.as_ref().unwrap().result, 0);
	let preview_ids = &start.enemy_deck_preview.as_ref().unwrap()[0].ship_ids;
	assert_eq!(preview_ids.len(), 1);
	assert!(matches!(preview_ids[0], 1501 | 1502 | 1503));

	let next = context.next_sortie(pid, Some(2)).await.unwrap();
	assert_eq!(next.from_cell_no, 1);
	assert_eq!(next.cell_no, 2);
	context.sortie_goback_port(pid).await.unwrap();
}

#[tokio::test]
async fn sortie_airbattle_reuses_single_fleet_day_battle_flow() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	let battle = context.sortie_airbattle(pid, 1).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert!(!battle.api_ship_ke.is_empty());
	context.sortie_goback_port(pid).await.unwrap();
}

#[tokio::test]
async fn sortie_goback_port_clears_pending_runtime_state() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_goback_port(pid).await.unwrap();

	assert!(context.sortie_battle_result(pid).await.is_err());
	assert!(context.next_sortie(pid, None).await.is_err());
}

#[tokio::test]
async fn sortie_battle_result_advances_generic_sortie_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 202;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_battle_result(pid).await.unwrap();

	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn sortie_battle_result_advances_boss_quest_on_real_boss_node() {
	let (context, session) = new_game_session_with_maps().await;
	let pid = session.profile.id;
	let quest_id = 204;
	let maparea_id = 1;
	let mapinfo_no = 2;
	let map_id = 12;

	let mut fleet_slots = [-1; 6];
	for slot in &mut fleet_slots {
		*slot = context.add_ship(pid, 951).await.unwrap().api_id;
	}
	context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let start = context.start_sortie(pid, 1, maparea_id, mapinfo_no, 1).await.unwrap();
	let path = path_to_boss(&context.1, map_id);
	assert_eq!(start.cell_no, path[0]);
	assert_eq!(start.boss_cell_no, *path.last().unwrap());
	for next_cell in path.iter().skip(1) {
		let next = context.next_sortie(pid, Some(*next_cell)).await.unwrap();
		assert_eq!(next.cell_no, *next_cell);
	}

	let battle = context.sortie_battle(pid, 1).await.unwrap();
	assert!(battle.api_eParam.iter().any(|param| param.iter().any(|value| *value > 0)));
	assert!(battle.api_eSlot.iter().any(|slots| slots.iter().any(|slot| *slot > 0)));
	context.sortie_battle_result(pid).await.unwrap();
	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn repo_wikiwiki_asset_supports_real_map_boss_progression() {
	let (context, session) = new_game_session_with_repo_wikiwiki_maps().await;
	let pid = session.profile.id;
	let quest_id = 204;
	let maparea_id = 1;
	let mapinfo_no = 2;
	let map_id = 12;

	let mut fleet_slots = [-1; 6];
	for slot in &mut fleet_slots {
		*slot = context.add_ship(pid, 951).await.unwrap().api_id;
	}
	context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let start = context.start_sortie(pid, 1, maparea_id, mapinfo_no, 1).await.unwrap();
	let path = path_to_boss(&context.1, map_id);
	assert_eq!(start.cell_no, path[0]);
	assert_eq!(start.boss_cell_no, *path.last().unwrap());
	for next_cell in path.iter().skip(1) {
		let next = context.next_sortie(pid, Some(*next_cell)).await.unwrap();
		assert_eq!(next.cell_no, *next_cell);
	}

	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_battle_result(pid).await.unwrap();
	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn sortie_battle_result_grants_ship_drop_from_repo_wikiwiki_map_catalog() {
	let (context, session) = new_game_session_with_repo_wikiwiki_maps().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	let definition = context.1.maps.map_definition(11).unwrap();
	let variant = definition.variant("").unwrap();
	let expected_drop_ship_ids = variant
		.ship_drops(start.cell_no)
		.unwrap()
		.iter()
		.filter(|drop| !drop.tags.iter().any(|tag| tag == "limited"))
		.map(|drop| drop.ship_id)
		.collect::<std::collections::BTreeSet<_>>();
	assert!(!expected_drop_ship_ids.is_empty());

	let ships_before = context.get_ships(pid).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	let result = context.sortie_battle_result(pid).await.unwrap();
	let drop = result.api_get_ship.as_ref().unwrap();
	let ships_after = context.get_ships(pid).await.unwrap();

	assert_eq!(result.api_get_flag, [0, 1, 0]);
	assert!(expected_drop_ship_ids.contains(&drop.api_ship_id));
	assert_eq!(ships_after.len(), ships_before.len() + 1);
	assert!(ships_after.iter().any(|ship| ship.api_ship_id == drop.api_ship_id));
}

#[tokio::test]
async fn sortie_goback_port_does_not_advance_sortie_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 202;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_goback_port(pid).await.unwrap();

	assert_eq!(quest_progress_of(&context, pid, quest_id).await, quest::progress::Progress::Empty);
}

#[tokio::test]
async fn monthly_map_record_resets_on_map_info_read() {
	let (db, mut codex) = mock_context().await;
	let map_id = 99001;
	codex.maps.maps.insert(
		map_id,
		MapDefinition {
			map_id,
			maparea_id: 99,
			mapinfo_no: 1,
			name: "Monthly Test Map".to_string(),
			level: 1,
			sally_flag: vec![],
			is_event: false,
			reset_policy: MapResetPolicy::Monthly,
			airbase_count: None,
			gauge_type: None,
			gauge_count: Some(1),
			required_defeat_count: Some(4),
			max_hp: None,
			default_variant: String::new(),
			rank_stage_ids: std::collections::BTreeMap::new(),
			variants: std::collections::BTreeMap::from([(
				String::new(),
				MapVariantDefinition {
					variant_key: String::new(),
					boss_cell_no: 1,
					cells: vec![],
					routing_rules: std::collections::BTreeMap::new(),
					enemy_fleets: std::collections::BTreeMap::new(),
					ship_drops: std::collections::BTreeMap::new(),
					required_defeat_count: Some(4),
					clear_to_variant_key: None,
					parse_warnings: vec![],
				},
			)]),
		},
	);
	codex.manifest.api_mst_mapinfo.push(ApiMstMapinfo {
		api_id: map_id,
		api_maparea_id: 99,
		api_no: 1,
		api_name: "Monthly Test Map".to_string(),
		api_level: 1,
		..ApiMstMapinfo::default()
	});
	let context = (db, codex);
	let account = context.sign_up("monthly-test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "monthly-admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
	let pid = session.profile.id;

	context.get_map_infos(pid).await.unwrap();

	let record = map_record::Entity::find()
		.filter(map_record::Column::ProfileId.eq(pid))
		.filter(map_record::Column::MapId.eq(map_id))
		.one(&context.0)
		.await
		.unwrap()
		.unwrap();
	let mut am = record.into_active_model();
	am.cleared = ActiveValue::Set(true);
	am.defeat_count = ActiveValue::Set(Some(4));
	am.last_reset_at = ActiveValue::Set(Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()));
	am.update(&context.0).await.unwrap();

	let infos = context.get_map_infos(pid).await.unwrap();
	let map_info = infos.into_iter().find(|info| info.api_id == map_id).unwrap();
	assert_eq!(map_info.api_cleared, 0);
	assert_eq!(map_info.api_defeat_count, Some(0));
}

#[tokio::test]
async fn combined_type_is_persisted_after_validation() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship1 = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship1.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	assert!(context.set_combined_type(pid, 1).await.is_err());

	context.unlock_fleet(pid, 2).await.unwrap();
	let ship2 = context.add_ship(pid, 952).await.unwrap();
	context.update_fleet_ships(pid, 2, &[ship2.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	assert_eq!(context.set_combined_type(pid, 1).await.unwrap(), 1);
	assert_eq!(context.get_combined_type(pid).await.unwrap(), 1);
}

#[tokio::test]
async fn sortie_battle_response_passes_battle_rule_validation() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	let battle = context.sortie_battle(pid, 1).await.unwrap();
	let assets = emukc_bootstrap::prelude::load_repo_battle_knowledge_assets().unwrap();
	let report = emukc_bootstrap::prelude::validate_day_battle_response(
		&context.1.manifest,
		&battle,
		&assets,
	)
	.unwrap();

	assert!(!report.has_errors(), "unexpected validation findings: {:?}", report.findings);
	assert!(
		!report.expected_resources.is_empty(),
		"validator should infer at least one expected resource"
	);
}

#[tokio::test]
async fn sortie_battle_validation_reports_invalid_enemy_ids() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	let battle = context.sortie_battle(pid, 1).await.unwrap();
	let mut raw = serde_json::to_value(&battle).unwrap();
	raw["api_ship_ke"][0] = serde_json::json!(999999);
	raw["api_eSlot"][0][0] = serde_json::json!(888888);

	let assets = emukc_bootstrap::prelude::load_repo_battle_knowledge_assets().unwrap();
	let report =
		emukc_bootstrap::prelude::validate_day_battle_response(&context.1.manifest, &raw, &assets)
			.unwrap();

	assert!(report.has_errors(), "mutated battle response should fail validation");
	assert!(report.findings.iter().any(|finding| {
		finding.kind == emukc_bootstrap::prelude::BattleValidationFindingKind::UnknownShipMstId
	}));
	assert!(report.findings.iter().any(|finding| {
		finding.kind == emukc_bootstrap::prelude::BattleValidationFindingKind::UnknownSlotitemMstId
	}));
}
