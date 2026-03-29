use std::{
	collections::HashMap,
	sync::{LazyLock, Mutex},
};

use async_trait::async_trait;
use emukc_db::sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*};
use emukc_model::{
	codex::{
		Codex,
		map::{EnemyComposition, EnemyFleetDefinition, MapDefinition, MapVariantDefinition},
	},
	kc2::{KcSortieResultRank, UserHQRank, level},
	thirdparty::QuestActionEvent,
};
use emukc_time::chrono::Utc;
use rand::{RngExt, rng};
use serde::Serialize;

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	basic::find_profile,
	battle::{
		core::{
			BattleContext, BattleMode, BattleNightHougeki, BattlePacket, BattleShipInput,
			EngagementType,
		},
		practice::PracticeBattleResponse,
		sortie::{
			SortieBattleInput, pending_sortie_battle, simulate_and_store_sortie_day_battle,
			simulate_and_store_sortie_night_battle, take_sortie_day_battle_result,
		},
	},
	fleet::get_fleet_ships_impl,
	map::{
		active_map_catalog, ensure_map_records_impl, find_map_definition, find_map_record_impl,
		refresh_all_map_records_impl,
	},
	quest::update::update_quest_progress_for_action,
	ship::update_ship_impl,
	slot_item::find_slot_items_by_id_impl,
};

static ACTIVE_SORTIES: LazyLock<Mutex<HashMap<i64, ActiveSortieState>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));
static PENDING_SORTIE_RESULTS: LazyLock<Mutex<HashMap<i64, SortieBattleResultSnapshot>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

pub type SortieBattleResponse = PracticeBattleResponse;

#[derive(Debug, Clone)]
struct ActiveSortieState {
	deck_id: i64,
	map_id: i64,
	map_name: String,
	map_level: i64,
	variant_key: String,
	current_cell_id: i64,
	boss_cell_id: i64,
	pending_battle_cell_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieStartCell {
	pub api_id: i64,
	pub api_no: i64,
	pub api_color_no: i64,
	pub api_passed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieStartResponse {
	pub api_cell_data: Vec<SortieStartCell>,
	pub api_rashin_flg: i64,
	pub api_rashin_id: i64,
	pub api_maparea_id: i64,
	pub api_mapinfo_no: i64,
	pub api_no: i64,
	pub api_color_no: i64,
	pub api_event_id: i64,
	pub api_event_kind: i64,
	pub api_next: i64,
	pub api_bosscell_no: i64,
	pub api_bosscomp: i64,
	pub api_from_no: i64,
	pub api_limit_state: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieNextResponse {
	pub api_rashin_flg: i64,
	pub api_rashin_id: i64,
	pub api_maparea_id: i64,
	pub api_mapinfo_no: i64,
	pub api_no: i64,
	pub api_color_no: i64,
	pub api_event_id: i64,
	pub api_event_kind: i64,
	pub api_next: i64,
	pub api_bosscell_no: i64,
	pub api_bosscomp: i64,
	pub api_from_no: i64,
}

#[derive(Debug, Clone)]
struct SortieBattleResultSnapshot {
	friendly_ship_ids: Vec<i64>,
	enemy_ship_ids: Vec<i64>,
	win_rank: String,
	get_exp: i64,
	member_lv: i64,
	member_exp: i64,
	get_base_exp: i64,
	mvp: i64,
	get_ship_exp: Vec<i64>,
	get_exp_lvup: Vec<Vec<i64>>,
	quest_name: String,
	quest_level: i64,
	enemy_level: i64,
	enemy_rank: String,
	enemy_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieBattleResultEnemyInfo {
	pub api_level: i64,
	pub api_rank: String,
	pub api_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieBattleResultResponse {
	pub api_ship_id: Vec<i64>,
	pub api_win_rank: String,
	pub api_get_exp: i64,
	pub api_mvp: i64,
	pub api_member_lv: i64,
	pub api_member_exp: i64,
	pub api_get_base_exp: i64,
	pub api_get_ship_exp: Vec<i64>,
	pub api_get_exp_lvup: Vec<Vec<i64>>,
	pub api_dests: i64,
	pub api_destsf: i64,
	pub api_quest_name: String,
	pub api_quest_level: i64,
	pub api_enemy_info: SortieBattleResultEnemyInfo,
	pub api_first_clear: i64,
	pub api_get_flag: [i64; 3],
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct SortieNightBattleResponse {
	pub api_deck_id: i64,
	pub api_formation: [i64; 3],
	pub api_f_nowhps: Vec<i64>,
	pub api_f_maxhps: Vec<i64>,
	pub api_fParam: Vec<[i64; 4]>,
	pub api_ship_ke: Vec<i64>,
	pub api_ship_lv: Vec<i64>,
	pub api_e_nowhps: Vec<i64>,
	pub api_e_maxhps: Vec<i64>,
	pub api_eSlot: Vec<[i64; 5]>,
	pub api_eParam: Vec<[i64; 4]>,
	pub api_smoke_type: i64,
	pub api_balloon_cell: i64,
	pub api_atoll_cell: i64,
	pub api_touch_plane: [i64; 2],
	pub api_flare_pos: [i64; 2],
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki: Option<BattleNightHougeki>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SortieGobackPortResponse {}

#[async_trait]
pub trait SortieOps {
	async fn start_sortie(
		&self,
		profile_id: i64,
		deck_id: i64,
		maparea_id: i64,
		mapinfo_no: i64,
		formation_id: i64,
	) -> Result<SortieStartResponse, GameplayError>;

	async fn next_sortie(
		&self,
		profile_id: i64,
		selected_cell_id: Option<i64>,
	) -> Result<SortieNextResponse, GameplayError>;

	async fn sortie_battle(
		&self,
		profile_id: i64,
		formation_id: i64,
	) -> Result<SortieBattleResponse, GameplayError>;

	async fn sortie_airbattle(
		&self,
		profile_id: i64,
		formation_id: i64,
	) -> Result<SortieBattleResponse, GameplayError>;

	async fn sortie_battle_result(
		&self,
		profile_id: i64,
	) -> Result<SortieBattleResultResponse, GameplayError>;

	async fn sortie_midnight_battle(
		&self,
		profile_id: i64,
	) -> Result<SortieNightBattleResponse, GameplayError>;

	async fn sortie_sp_midnight_battle(
		&self,
		profile_id: i64,
	) -> Result<SortieNightBattleResponse, GameplayError>;

	async fn sortie_goback_port(
		&self,
		profile_id: i64,
	) -> Result<SortieGobackPortResponse, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> SortieOps for T {
	async fn start_sortie(
		&self,
		profile_id: i64,
		deck_id: i64,
		maparea_id: i64,
		mapinfo_no: i64,
		formation_id: i64,
	) -> Result<SortieStartResponse, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		find_profile(&tx, profile_id).await?;
		let fleet_ships = get_fleet_ships_impl(&tx, profile_id, deck_id).await?;
		if fleet_ships.is_empty() {
			return Err(GameplayError::WrongType(format!(
				"fleet {deck_id} has no ships for sortie",
			)));
		}

		ensure_map_records_impl(&tx, codex, profile_id).await?;
		refresh_all_map_records_impl(&tx, codex, profile_id).await?;
		let definition = find_map_definition(codex, maparea_id, mapinfo_no)?;
		let record = find_map_record_impl(&tx, profile_id, definition.map_id).await?;
		let variant_key = record.variant_key.clone().unwrap_or_default();
		let variant = definition.variant(&variant_key).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"variant `{variant_key}` not found for map {}",
				definition.map_id,
			))
		})?;
		let first_cell = variant.first_progress_cell_no().ok_or_else(|| {
			GameplayError::WrongType(format!(
				"map {} has no navigable first cell",
				definition.map_id,
			))
		})?;
		let current_cell = variant
			.cell(first_cell)
			.ok_or_else(|| GameplayError::EntryNotFound(format!("cell {first_cell} not found")))?;

		let active = ActiveSortieState {
			deck_id,
			map_id: definition.map_id,
			map_name: definition.name.clone(),
			map_level: definition.level,
			variant_key,
			current_cell_id: first_cell,
			boss_cell_id: variant.boss_cell_no,
			pending_battle_cell_id: None,
		};
		ACTIVE_SORTIES.lock().unwrap().insert(profile_id, active);
		tx.commit().await?;

		let _ = formation_id;
		Ok(SortieStartResponse {
			api_cell_data: build_sortie_cell_data(definition.map_id, variant),
			api_rashin_flg: 0,
			api_rashin_id: 0,
			api_maparea_id: maparea_id,
			api_mapinfo_no: mapinfo_no,
			api_no: current_cell.cell_no,
			api_color_no: current_cell.color_no,
			api_event_id: current_cell.event_id,
			api_event_kind: current_cell.event_kind,
			api_next: i64::from(!current_cell.next_cells.is_empty()),
			api_bosscell_no: variant.boss_cell_no,
			api_bosscomp: i64::from(current_cell.cell_no == variant.boss_cell_no),
			api_from_no: 0,
			api_limit_state: 0,
		})
	}

	async fn next_sortie(
		&self,
		profile_id: i64,
		selected_cell_id: Option<i64>,
	) -> Result<SortieNextResponse, GameplayError> {
		let codex = self.codex();
		let active = ACTIVE_SORTIES.lock().unwrap().get(&profile_id).cloned().ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"active sortie not found for profile {profile_id}",
			))
		})?;
		if active.pending_battle_cell_id.is_some() {
			return Err(GameplayError::WrongType(
				"cannot advance sortie while a battle result is pending".to_string(),
			));
		}

		let catalog = active_map_catalog(codex);
		let definition = catalog.map_definition(active.map_id).cloned().ok_or_else(|| {
			GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
		})?;
		let variant = definition.variant(&active.variant_key).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"variant `{}` not found for map {}",
				active.variant_key, active.map_id,
			))
		})?;
		let current = variant.cell(active.current_cell_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"cell {} not found in map {}",
				active.current_cell_id, active.map_id,
			))
		})?;
		if current.next_cells.is_empty() {
			return Err(GameplayError::WrongType(format!(
				"cell {} has no next route",
				current.cell_no,
			)));
		}

		let next_cell_id = if let Some(selected_cell_id) = selected_cell_id {
			if !current.next_cells.contains(&selected_cell_id) {
				return Err(GameplayError::WrongType(format!(
					"cell {selected_cell_id} is not a valid route from {}",
					current.cell_no,
				)));
			}
			selected_cell_id
		} else {
			current.next_cells[0]
		};
		let next = variant.cell(next_cell_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!("cell {next_cell_id} not found"))
		})?;

		ACTIVE_SORTIES.lock().unwrap().entry(profile_id).and_modify(|state| {
			state.current_cell_id = next_cell_id;
		});

		let (maparea_id, mapinfo_no) = split_map_id(active.map_id);
		Ok(SortieNextResponse {
			api_rashin_flg: i64::from(current.next_cells.len() > 1),
			api_rashin_id: if current.next_cells.len() > 1 {
				1
			} else {
				0
			},
			api_maparea_id: maparea_id,
			api_mapinfo_no: mapinfo_no,
			api_no: next.cell_no,
			api_color_no: next.color_no,
			api_event_id: next.event_id,
			api_event_kind: next.event_kind,
			api_next: i64::from(!next.next_cells.is_empty()),
			api_bosscell_no: variant.boss_cell_no,
			api_bosscomp: i64::from(next.cell_no == variant.boss_cell_no),
			api_from_no: current.cell_no,
		})
	}

	async fn sortie_battle(
		&self,
		profile_id: i64,
		formation_id: i64,
	) -> Result<SortieBattleResponse, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let mut active =
			ACTIVE_SORTIES.lock().unwrap().get(&profile_id).cloned().ok_or_else(|| {
				GameplayError::EntryNotFound(format!(
					"active sortie not found for profile {profile_id}",
				))
			})?;
		if active.pending_battle_cell_id.is_some() {
			return Err(GameplayError::WrongType("sortie battle already pending".to_string()));
		}

		let profile = find_profile(&tx, profile_id).await?;
		if profile.combined_type > 0 {
			return Err(GameplayError::WrongType(
				"combined sortie battle is not implemented yet".to_string(),
			));
		}

		let catalog = active_map_catalog(codex);
		let definition = catalog.map_definition(active.map_id).cloned().ok_or_else(|| {
			GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
		})?;
		let variant = definition.variant(&active.variant_key).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"variant `{}` not found for map {}",
				active.variant_key, active.map_id,
			))
		})?;
		let current_cell = variant.cell(active.current_cell_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"cell {} not found in map {}",
				active.current_cell_id, active.map_id,
			))
		})?;
		if current_cell.event_kind != 1 {
			return Err(GameplayError::WrongType(format!(
				"cell {} is not a battle cell",
				current_cell.cell_no,
			)));
		}

		let fleet_ships = get_fleet_ships_impl(&tx, profile_id, active.deck_id).await?;
		if fleet_ships.is_empty() {
			return Err(GameplayError::WrongType(format!(
				"fleet {} has no ships for sortie battle",
				active.deck_id,
			)));
		}

		let friend_ships = build_sortie_friend_ships(&tx, &fleet_ships).await?;
		let enemy_fleet = resolve_sortie_enemy_fleet(active.map_id, variant, current_cell.cell_no);
		let enemy_composition = select_random_enemy_composition(&enemy_fleet)
			.unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
		let (enemy_ships, enemy_level, enemy_rank, enemy_deck_name) =
			build_sortie_enemy_ships(codex, &definition, &enemy_fleet, &enemy_composition)?;

		let session = simulate_and_store_sortie_day_battle(
			codex,
			SortieBattleInput {
				profile_id,
				deck_id: active.deck_id,
				map_id: active.map_id,
				cell_id: active.current_cell_id,
				context: BattleContext {
					mode: BattleMode::Sortie,
					friendly_formation_id: formation_id,
					enemy_formation_id: enemy_fleet.formations.first().copied().unwrap_or(1),
					engagement: engagement_for_cell(active.map_id, active.current_cell_id),
					friend_ships: friend_ships.clone(),
					enemy_ships: enemy_ships.clone(),
					rng_seed: None,
				},
			},
		);

		let base_exp = calculate_sortie_base_exp(active.map_level, active.current_cell_id);
		let get_exp = calculate_battle_admiral_exp(base_exp, &session.outcome.win_rank);
		let (ship_exp, ship_lvup) =
			calculate_sortie_ship_exp(&friend_ships, base_exp, session.outcome.mvp);
		let response = build_sortie_battle_response(
			active.deck_id,
			friend_ships,
			enemy_ships,
			session.packet.clone(),
		);
		PENDING_SORTIE_RESULTS.lock().unwrap().insert(
			profile_id,
			SortieBattleResultSnapshot {
				friendly_ship_ids: session.friendly_ship_ids.clone(),
				enemy_ship_ids: session.enemy_ship_ids.clone(),
				win_rank: session.outcome.win_rank.clone(),
				get_exp,
				member_lv: profile.hq_level,
				member_exp: profile.experience,
				get_base_exp: base_exp,
				mvp: session.outcome.mvp,
				get_ship_exp: ship_exp,
				get_exp_lvup: ship_lvup,
				quest_name: active.map_name.clone(),
				quest_level: active.map_level,
				enemy_level,
				enemy_rank,
				enemy_deck_name,
			},
		);

		active.pending_battle_cell_id = Some(active.current_cell_id);
		ACTIVE_SORTIES.lock().unwrap().insert(profile_id, active);

		tx.commit().await?;
		Ok(response)
	}

	async fn sortie_airbattle(
		&self,
		profile_id: i64,
		formation_id: i64,
	) -> Result<SortieBattleResponse, GameplayError> {
		self.sortie_battle(profile_id, formation_id).await
	}

	async fn sortie_battle_result(
		&self,
		profile_id: i64,
	) -> Result<SortieBattleResultResponse, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let snapshot =
			PENDING_SORTIE_RESULTS.lock().unwrap().remove(&profile_id).ok_or_else(|| {
				GameplayError::EntryNotFound(format!(
					"sortie battle result not found for profile {profile_id}",
				))
			})?;
		let session = take_sortie_day_battle_result(profile_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"sortie battle session not found for profile {profile_id}",
			))
		})?;
		let active = ACTIVE_SORTIES.lock().unwrap().get(&profile_id).cloned().ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"active sortie not found for profile {profile_id}",
			))
		})?;
		let pending_cell_id = active.pending_battle_cell_id.ok_or_else(|| {
			GameplayError::WrongType("no pending sortie battle to resolve".to_string())
		})?;

		let catalog = active_map_catalog(codex);
		let definition = catalog.map_definition(active.map_id).cloned().ok_or_else(|| {
			GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
		})?;
		let variant = definition.variant(&active.variant_key).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"variant `{}` not found for map {}",
				active.variant_key, active.map_id,
			))
		})?;
		let current_cell = variant.cell(pending_cell_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!("cell {pending_cell_id} not found"))
		})?;

		let snapshot = update_sortie_result_stats(&tx, codex, profile_id, snapshot).await?;
		let first_clear = apply_sortie_map_result(
			&tx,
			profile_id,
			&definition,
			current_cell.cell_no == active.boss_cell_id,
			&snapshot,
		)
		.await?;
		let quest_event = build_sortie_quest_event(&definition, &active, &snapshot)?;
		update_quest_progress_for_action(&tx, codex, profile_id, &quest_event).await?;

		tx.commit().await?;

		let should_finish_sortie =
			current_cell.cell_no == active.boss_cell_id || current_cell.next_cells.is_empty();
		if should_finish_sortie {
			ACTIVE_SORTIES.lock().unwrap().remove(&profile_id);
		} else {
			ACTIVE_SORTIES.lock().unwrap().entry(profile_id).and_modify(|state| {
				state.pending_battle_cell_id = None;
			});
		}

		Ok(SortieBattleResultResponse {
			api_ship_id: snapshot.enemy_ship_ids,
			api_win_rank: snapshot.win_rank,
			api_get_exp: snapshot.get_exp,
			api_mvp: snapshot.mvp,
			api_member_lv: snapshot.member_lv,
			api_member_exp: snapshot.member_exp,
			api_get_base_exp: snapshot.get_base_exp,
			api_get_ship_exp: snapshot.get_ship_exp,
			api_get_exp_lvup: snapshot.get_exp_lvup,
			api_dests: session.packet.enemy_nowhps.iter().filter(|hp| **hp <= 0).count() as i64,
			api_destsf: i64::from(session.packet.enemy_nowhps.first().copied().unwrap_or(1) <= 0),
			api_quest_name: snapshot.quest_name,
			api_quest_level: snapshot.quest_level,
			api_enemy_info: SortieBattleResultEnemyInfo {
				api_level: snapshot.enemy_level,
				api_rank: snapshot.enemy_rank,
				api_deck_name: snapshot.enemy_deck_name,
			},
			api_first_clear: first_clear,
			api_get_flag: [0, 0, 0],
		})
	}

	async fn sortie_midnight_battle(
		&self,
		profile_id: i64,
	) -> Result<SortieNightBattleResponse, GameplayError> {
		let codex = self.codex();
		let pending = pending_sortie_battle(profile_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"sortie battle session not found for profile {profile_id}",
			))
		})?;
		if !pending.outcome.can_midnight {
			return Err(GameplayError::WrongType(
				"night battle is not available for this sortie battle".to_string(),
			));
		}

		let night = simulate_and_store_sortie_night_battle(
			codex,
			profile_id,
			pending.packet.formation[0],
			pending.packet.formation[1],
			EngagementType::from_api_id(pending.packet.formation[2])
				.unwrap_or(EngagementType::SameCourse),
		)
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"sortie battle session not found for profile {profile_id}",
			))
		})?;

		if let Some(snapshot) = PENDING_SORTIE_RESULTS.lock().unwrap().get_mut(&profile_id) {
			snapshot.win_rank = night.outcome.win_rank.clone();
			snapshot.mvp = night.outcome.mvp;
			snapshot.get_exp =
				calculate_battle_admiral_exp(snapshot.get_base_exp, &snapshot.win_rank);
			if let Some(updated) = pending_sortie_battle(profile_id) {
				let friend_ships = updated
					.friendly
					.iter()
					.cloned()
					.map(|ship| BattleShipInput {
						ship: ship.ship,
						slot_items: ship.slot_items,
						effect_list: ship.effect_list,
					})
					.collect::<Vec<_>>();
				let (ship_exp, ship_lvup) =
					calculate_sortie_ship_exp(&friend_ships, snapshot.get_base_exp, snapshot.mvp);
				snapshot.get_ship_exp = ship_exp;
				snapshot.get_exp_lvup = ship_lvup;
			}
		}

		let current = pending_sortie_battle(profile_id).ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"sortie battle session not found for profile {profile_id}",
			))
		})?;
		Ok(build_sortie_night_battle_response(current.deck_id, &current, night.packet))
	}

	async fn sortie_sp_midnight_battle(
		&self,
		profile_id: i64,
	) -> Result<SortieNightBattleResponse, GameplayError> {
		self.sortie_midnight_battle(profile_id).await
	}

	async fn sortie_goback_port(
		&self,
		profile_id: i64,
	) -> Result<SortieGobackPortResponse, GameplayError> {
		let removed = ACTIVE_SORTIES.lock().unwrap().remove(&profile_id);
		if removed.is_none() {
			return Err(GameplayError::EntryNotFound(format!(
				"active sortie not found for profile {profile_id}",
			)));
		}

		clear_pending_sortie_runtime_state(profile_id);

		Ok(SortieGobackPortResponse::default())
	}
}

pub(crate) fn split_map_id(map_id: i64) -> (i64, i64) {
	(map_id / 10, map_id % 10)
}

fn build_sortie_cell_data(map_id: i64, variant: &MapVariantDefinition) -> Vec<SortieStartCell> {
	variant
		.cells
		.iter()
		.map(|cell| SortieStartCell {
			api_id: map_id * 100 + cell.cell_no,
			api_no: cell.cell_no,
			api_color_no: cell.color_no,
			api_passed: 0,
		})
		.collect()
}

async fn build_sortie_friend_ships<C>(
	c: &C,
	friend_ships: &[emukc_db::entity::profile::ship::Model],
) -> Result<Vec<BattleShipInput>, GameplayError>
where
	C: ConnectionTrait,
{
	let mut result = Vec::with_capacity(friend_ships.len());
	for ship in friend_ships {
		let slot_ids =
			[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
				.into_iter()
				.filter(|slot_id| *slot_id > 0)
				.collect::<Vec<_>>();
		let slot_items = find_slot_items_by_id_impl(c, &slot_ids).await?;
		let slot_items = slot_items.into_iter().map(std::convert::Into::into).collect();

		result.push(BattleShipInput {
			ship: (*ship).into(),
			slot_items,
			effect_list: vec![],
		});
	}

	Ok(result)
}

fn build_sortie_enemy_ships(
	codex: &Codex,
	definition: &MapDefinition,
	enemy_fleet: &EnemyFleetDefinition,
	composition: &EnemyComposition,
) -> Result<(Vec<BattleShipInput>, i64, String, String), GameplayError> {
	let enemy_level = (definition.level.max(1) * 5 + enemy_fleet.cell_no).max(1);
	let enemy_rank = UserHQRank::RearAdmiral.get_name().to_string();
	let enemy_deck_name = format!("{}海域敵艦隊", definition.name);
	let ship_ids = if composition.ship_ids.is_empty() {
		vec![412]
	} else {
		composition.ship_ids.clone()
	};

	let enemy_ships = ship_ids
		.into_iter()
		.map(|ship_id| {
			let (mut api_ship, slot_items) =
				codex.new_ship(ship_id).ok_or(GameplayError::ManifestNotFound(ship_id))?;
			let exp_now = level::ship_level_required_exp(enemy_level.min(99));
			let (_, next_exp) = level::exp_to_ship_level(exp_now);
			api_ship.api_lv = enemy_level;
			api_ship.api_exp = [exp_now, next_exp, 0];
			codex.cal_ship_status(&mut api_ship, &slot_items)?;

			Ok(BattleShipInput {
				ship: api_ship,
				slot_items,
				effect_list: vec![0],
			})
		})
		.collect::<Result<Vec<_>, GameplayError>>()?;

	Ok((enemy_ships, enemy_level, enemy_rank, enemy_deck_name))
}

fn resolve_sortie_enemy_fleet(
	map_id: i64,
	variant: &MapVariantDefinition,
	cell_no: i64,
) -> EnemyFleetDefinition {
	if let Some(enemy_fleet) = variant.enemy_fleets.get(&cell_no) {
		return enemy_fleet.clone();
	}

	warn!(
		map_id,
		cell_no, "missing enemy fleet definition for sortie cell; using fallback composition",
	);
	fallback_enemy_fleet(cell_no)
}

fn fallback_enemy_fleet(cell_no: i64) -> EnemyFleetDefinition {
	EnemyFleetDefinition {
		cell_no,
		battle_kind: 1,
		formations: vec![1],
		compositions: vec![fallback_enemy_composition(cell_no)],
	}
}

fn fallback_enemy_composition(cell_no: i64) -> EnemyComposition {
	EnemyComposition {
		comp_id: format!("fallback:{cell_no}"),
		weight: 1,
		ship_ids: vec![412],
	}
}

fn select_random_enemy_composition(enemy_fleet: &EnemyFleetDefinition) -> Option<EnemyComposition> {
	if enemy_fleet.compositions.is_empty() {
		return None;
	}

	let total_weight = enemy_fleet
		.compositions
		.iter()
		.map(|composition| composition.weight.max(1) as u64)
		.sum::<u64>();
	if total_weight == 0 {
		return enemy_fleet.compositions.first().cloned();
	}

	let mut random = rng();
	let roll = random.random_range(0..total_weight);
	select_enemy_composition_for_roll(enemy_fleet, roll).cloned()
}

fn select_enemy_composition_for_roll(
	enemy_fleet: &EnemyFleetDefinition,
	mut roll: u64,
) -> Option<&EnemyComposition> {
	for composition in &enemy_fleet.compositions {
		let weight = composition.weight.max(1) as u64;
		if roll < weight {
			return Some(composition);
		}
		roll -= weight;
	}

	enemy_fleet.compositions.last()
}

fn clear_pending_sortie_runtime_state(profile_id: i64) {
	PENDING_SORTIE_RESULTS.lock().unwrap().remove(&profile_id);
	let _ = take_sortie_day_battle_result(profile_id);
}

fn build_sortie_battle_response(
	deck_id: i64,
	friend_ships: Vec<BattleShipInput>,
	enemy_ships: Vec<BattleShipInput>,
	packet: BattlePacket,
) -> SortieBattleResponse {
	SortieBattleResponse {
		api_deck_id: deck_id,
		api_formation: packet.formation,
		api_f_nowhps: friend_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
		api_f_maxhps: friend_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
		api_fParam: friend_ships
			.iter()
			.map(|ship| {
				[
					ship.ship.api_karyoku[0],
					ship.ship.api_raisou[0],
					ship.ship.api_taiku[0],
					ship.ship.api_soukou[0],
				]
			})
			.collect(),
		api_ship_ke: enemy_ships.iter().map(|ship| ship.ship.api_ship_id).collect(),
		api_ship_lv: enemy_ships.iter().map(|ship| ship.ship.api_lv).collect(),
		api_e_nowhps: enemy_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
		api_e_maxhps: enemy_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
		api_eSlot: enemy_ships.iter().map(enemy_slot_ids).collect(),
		api_eParam: enemy_ships
			.iter()
			.map(|ship| {
				[
					ship.ship.api_karyoku[0],
					ship.ship.api_raisou[0],
					ship.ship.api_taiku[0],
					ship.ship.api_soukou[0],
				]
			})
			.collect(),
		api_e_effect_list: enemy_ships
			.iter()
			.map(|ship| {
				if ship.effect_list.is_empty() {
					vec![0]
				} else {
					ship.effect_list.clone()
				}
			})
			.collect(),
		api_smoke_type: packet.smoke_type,
		api_balloon_cell: packet.balloon_cell,
		api_atoll_cell: packet.atoll_cell,
		api_midnight_flag: packet.midnight_flag,
		api_search: packet.search,
		api_stage_flag: packet.stage_flag,
		api_kouku: packet.kouku,
		api_opening_taisen_flag: packet.opening_taisen_flag,
		api_opening_taisen: packet.opening_taisen,
		api_opening_flag: packet.opening_flag,
		api_opening_atack: packet.opening_attack,
		api_hourai_flag: packet.hourai_flag,
		api_hougeki1: packet.hougeki1,
		api_hougeki2: packet.hougeki2,
		api_hougeki3: packet.hougeki3,
		api_raigeki: packet.raigeki,
	}
}

fn build_sortie_night_battle_response(
	deck_id: i64,
	session: &crate::game::battle::sortie::SortieBattleSession,
	packet: crate::game::battle::core::NightBattlePacket,
) -> SortieNightBattleResponse {
	SortieNightBattleResponse {
		api_deck_id: deck_id,
		api_formation: packet.formation,
		api_f_nowhps: packet.friendly_nowhps,
		api_f_maxhps: packet.friendly_maxhps,
		api_fParam: session
			.friendly
			.iter()
			.map(|ship| {
				[
					ship.ship.api_karyoku[0],
					ship.ship.api_raisou[0],
					ship.ship.api_taiku[0],
					ship.ship.api_soukou[0],
				]
			})
			.collect(),
		api_ship_ke: session.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		api_ship_lv: session.enemy.iter().map(|ship| ship.ship.api_lv).collect(),
		api_e_nowhps: packet.enemy_nowhps,
		api_e_maxhps: packet.enemy_maxhps,
		api_eSlot: session
			.enemy
			.iter()
			.map(|ship| {
				enemy_slot_ids(&BattleShipInput {
					ship: ship.ship.clone(),
					slot_items: ship.slot_items.clone(),
					effect_list: ship.effect_list.clone(),
				})
			})
			.collect(),
		api_eParam: session
			.enemy
			.iter()
			.map(|ship| {
				[
					ship.ship.api_karyoku[0],
					ship.ship.api_raisou[0],
					ship.ship.api_taiku[0],
					ship.ship.api_soukou[0],
				]
			})
			.collect(),
		api_smoke_type: 0,
		api_balloon_cell: 0,
		api_atoll_cell: 0,
		api_touch_plane: packet.touch_plane,
		api_flare_pos: packet.flare_pos,
		api_hougeki: packet.hougeki,
	}
}

fn build_sortie_quest_event(
	definition: &MapDefinition,
	active: &ActiveSortieState,
	snapshot: &SortieBattleResultSnapshot,
) -> Result<QuestActionEvent, GameplayError> {
	Ok(QuestActionEvent::SortieBattleCompleted {
		maparea_id: definition.maparea_id,
		mapinfo_no: definition.mapinfo_no,
		boss_cell: active.pending_battle_cell_id == Some(active.boss_cell_id),
		win_rank: parse_sortie_result_rank(&snapshot.win_rank)?,
		fleet_id: active.deck_id,
	})
}

fn parse_sortie_result_rank(win_rank: &str) -> Result<KcSortieResultRank, GameplayError> {
	match win_rank {
		"S" => Ok(KcSortieResultRank::S),
		"A" => Ok(KcSortieResultRank::A),
		"B" => Ok(KcSortieResultRank::B),
		"C" => Ok(KcSortieResultRank::C),
		"D" => Ok(KcSortieResultRank::D),
		"E" => Ok(KcSortieResultRank::E),
		_ => Err(GameplayError::WrongType(format!("unexpected sortie result rank `{win_rank}`",))),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::game::battle::{core::BattleMode, sortie::simulate_and_store_sortie_day_battle};
	use emukc_db::prelude::new_mem_db;
	use emukc_model::codex::Codex;

	fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
		let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
		let exp_now = level::ship_level_required_exp(level);
		let (_, next_exp) = level::exp_to_ship_level(exp_now);
		ship.api_lv = level;
		ship.api_exp = [exp_now, next_exp, 0];
		codex.cal_ship_status(&mut ship, &slot_items).unwrap();
		BattleShipInput {
			ship,
			slot_items,
			effect_list: vec![0],
		}
	}

	fn weaken_for_midnight(mut ship: BattleShipInput) -> BattleShipInput {
		ship.ship.api_karyoku[0] = 1;
		ship.ship.api_raisou[0] = 0;
		ship.ship.api_soukou[0] = 200;
		ship
	}

	#[tokio::test]
	async fn sortie_midnight_battle_updates_pending_snapshot() {
		ACTIVE_SORTIES.lock().unwrap().clear();
		PENDING_SORTIE_RESULTS.lock().unwrap().clear();

		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let context = (db, codex.clone());
		let profile_id = 42;

		let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
		let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));
		let session = simulate_and_store_sortie_day_battle(
			&codex,
			SortieBattleInput {
				profile_id,
				deck_id: 1,
				map_id: 11,
				cell_id: 1,
				context: BattleContext {
					mode: BattleMode::Sortie,
					friendly_formation_id: 1,
					enemy_formation_id: 1,
					engagement: EngagementType::SameCourse,
					friend_ships: vec![friend.clone()],
					enemy_ships: vec![enemy.clone()],
					rng_seed: Some(1),
				},
			},
		);

		assert_eq!(session.packet.midnight_flag, 1);
		PENDING_SORTIE_RESULTS.lock().unwrap().insert(
			profile_id,
			SortieBattleResultSnapshot {
				friendly_ship_ids: session.friendly_ship_ids.clone(),
				enemy_ship_ids: session.enemy_ship_ids.clone(),
				win_rank: session.outcome.win_rank.clone(),
				get_exp: 0,
				member_lv: 1,
				member_exp: 0,
				get_base_exp: 30,
				mvp: session.outcome.mvp,
				get_ship_exp: vec![],
				get_exp_lvup: vec![],
				quest_name: "test".to_string(),
				quest_level: 1,
				enemy_level: 1,
				enemy_rank: "Test".to_string(),
				enemy_deck_name: "Test".to_string(),
			},
		);

		let response = context.sortie_midnight_battle(profile_id).await.unwrap();
		assert_eq!(response.api_deck_id, 1);
		assert!(response.api_hougeki.is_some());

		let updated_snapshot =
			PENDING_SORTIE_RESULTS.lock().unwrap().get(&profile_id).cloned().unwrap();
		assert!(!updated_snapshot.win_rank.is_empty());
		assert!(updated_snapshot.mvp >= 1);

		let stored = pending_sortie_battle(profile_id).unwrap();
		assert_eq!(stored.packet.midnight_flag, 0);

		let _ = take_sortie_day_battle_result(profile_id);
		PENDING_SORTIE_RESULTS.lock().unwrap().clear();
	}

	#[tokio::test]
	async fn sortie_sp_midnight_battle_uses_existing_night_flow() {
		ACTIVE_SORTIES.lock().unwrap().clear();
		PENDING_SORTIE_RESULTS.lock().unwrap().clear();

		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let context = (db, codex.clone());
		let profile_id = 84;

		let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
		let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));
		let session = simulate_and_store_sortie_day_battle(
			&codex,
			SortieBattleInput {
				profile_id,
				deck_id: 1,
				map_id: 11,
				cell_id: 1,
				context: BattleContext {
					mode: BattleMode::Sortie,
					friendly_formation_id: 1,
					enemy_formation_id: 1,
					engagement: EngagementType::SameCourse,
					friend_ships: vec![friend.clone()],
					enemy_ships: vec![enemy.clone()],
					rng_seed: Some(1),
				},
			},
		);

		PENDING_SORTIE_RESULTS.lock().unwrap().insert(
			profile_id,
			SortieBattleResultSnapshot {
				friendly_ship_ids: session.friendly_ship_ids.clone(),
				enemy_ship_ids: session.enemy_ship_ids.clone(),
				win_rank: session.outcome.win_rank.clone(),
				get_exp: 0,
				member_lv: 1,
				member_exp: 0,
				get_base_exp: 30,
				mvp: session.outcome.mvp,
				get_ship_exp: vec![],
				get_exp_lvup: vec![],
				quest_name: "test".to_string(),
				quest_level: 1,
				enemy_level: 1,
				enemy_rank: "Test".to_string(),
				enemy_deck_name: "Test".to_string(),
			},
		);

		let response = context.sortie_sp_midnight_battle(profile_id).await.unwrap();
		assert_eq!(response.api_deck_id, 1);
		assert!(response.api_hougeki.is_some());

		clear_pending_sortie_runtime_state(profile_id);
	}

	#[test]
	fn weighted_enemy_composition_selection_uses_weights() {
		let enemy_fleet = EnemyFleetDefinition {
			cell_no: 3,
			battle_kind: 1,
			formations: vec![1],
			compositions: vec![
				EnemyComposition {
					comp_id: "light".to_string(),
					weight: 1,
					ship_ids: vec![501],
				},
				EnemyComposition {
					comp_id: "heavy".to_string(),
					weight: 3,
					ship_ids: vec![502],
				},
			],
		};

		assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 0).unwrap().comp_id, "light",);
		assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 1).unwrap().comp_id, "heavy",);
		assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 3).unwrap().comp_id, "heavy",);
	}

	#[test]
	fn fallback_enemy_fleet_is_only_used_when_catalog_data_is_missing() {
		let mut variant = MapVariantDefinition {
			variant_key: String::new(),
			boss_cell_no: 5,
			cells: vec![],
			enemy_fleets: HashMap::new().into_iter().collect(),
		};
		variant.enemy_fleets.insert(
			2,
			EnemyFleetDefinition {
				cell_no: 2,
				battle_kind: 1,
				formations: vec![2],
				compositions: vec![EnemyComposition {
					comp_id: "real".to_string(),
					weight: 1,
					ship_ids: vec![501, 502],
				}],
			},
		);

		let real = resolve_sortie_enemy_fleet(11, &variant, 2);
		assert_eq!(real.formations, vec![2]);
		assert_eq!(real.compositions[0].ship_ids, vec![501, 502]);

		let fallback = resolve_sortie_enemy_fleet(11, &variant, 7);
		assert_eq!(fallback.compositions[0].ship_ids, vec![412]);
	}
}

fn enemy_slot_ids(ship: &BattleShipInput) -> [i64; 5] {
	let mut slots = [-1; 5];
	for (idx, slot_item) in ship.slot_items.iter().take(5).enumerate() {
		slots[idx] = slot_item.api_slotitem_id;
	}
	slots
}

fn calculate_sortie_base_exp(map_level: i64, cell_id: i64) -> i64 {
	(map_level.max(1) * 25 + cell_id * 10).clamp(30, 1200)
}

fn calculate_battle_admiral_exp(base_exp: i64, win_rank: &str) -> i64 {
	match win_rank {
		"S" => (base_exp as f64 * 0.12).round() as i64,
		"A" => (base_exp as f64 * 0.1).round() as i64,
		"B" => (base_exp as f64 * 0.08).round() as i64,
		"C" => (base_exp as f64 * 0.05).round() as i64,
		_ => (base_exp as f64 * 0.03).round() as i64,
	}
}

fn calculate_sortie_ship_exp(
	friend_ships: &[BattleShipInput],
	base_exp: i64,
	mvp_idx: i64,
) -> (Vec<i64>, Vec<Vec<i64>>) {
	let mut exp = vec![-1];
	let mut lvup = Vec::with_capacity(friend_ships.len());

	for (idx, ship) in friend_ships.iter().enumerate() {
		let gain = if idx as i64 + 1 == mvp_idx {
			base_exp * 2
		} else if idx == 0 {
			((base_exp as f64) * 1.5).floor() as i64
		} else {
			base_exp
		};
		exp.push(gain);

		let new_exp = ship.ship.api_exp[0] + gain;
		lvup.push(build_exp_lvup_vector(ship.ship.api_exp[0], new_exp));
	}

	(exp, lvup)
}

fn build_exp_lvup_vector(before_exp: i64, after_exp: i64) -> Vec<i64> {
	let mut result = vec![before_exp];
	let (_, mut next_exp) = level::exp_to_ship_level(before_exp);
	if next_exp <= 0 {
		result.push(-1);
		return result;
	}
	result.push(next_exp);

	while next_exp > 0 && after_exp >= next_exp {
		let (_, candidate_next) = level::exp_to_ship_level(next_exp);
		if candidate_next <= 0 {
			result.push(-1);
			break;
		}
		if candidate_next == next_exp {
			break;
		}
		result.push(candidate_next);
		next_exp = candidate_next;
	}

	result
}

fn engagement_for_cell(map_id: i64, cell_id: i64) -> EngagementType {
	match (map_id + cell_id).rem_euclid(4) {
		1 => EngagementType::HeadOn,
		2 => EngagementType::TAdvantage,
		3 => EngagementType::TDisadvantage,
		_ => EngagementType::SameCourse,
	}
}

async fn update_sortie_result_stats<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	mut snapshot: SortieBattleResultSnapshot,
) -> Result<SortieBattleResultSnapshot, GameplayError>
where
	C: ConnectionTrait,
{
	let profile = find_profile(c, profile_id).await?;
	let mut am = profile.into_active_model();
	let current_exp = am.experience.take().unwrap_or_default();
	let new_exp = current_exp + snapshot.get_exp;
	let (hq_level, _) = level::exp_to_hq_level(new_exp);
	if matches!(snapshot.win_rank.as_str(), "S" | "A" | "B") {
		am.sortie_wins = ActiveValue::Set(am.sortie_wins.take().unwrap_or_default() + 1);
	} else {
		am.sortie_loses = ActiveValue::Set(am.sortie_loses.take().unwrap_or_default() + 1);
	}
	am.experience = ActiveValue::Set(new_exp);
	am.hq_level = ActiveValue::Set(hq_level);
	let updated_profile = am.update(c).await?;

	for (idx, ship_id) in snapshot.friendly_ship_ids.iter().copied().enumerate() {
		let gain = snapshot.get_ship_exp.get(idx + 1).copied().unwrap_or(-1);
		if gain <= 0 {
			continue;
		}
		let ship_model =
			emukc_db::entity::profile::ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(
				|| GameplayError::EntryNotFound(format!("ship with id {ship_id} not found")),
			)?;
		let mut api_ship: emukc_model::kc2::KcApiShip = ship_model.clone().into();
		let new_ship_exp = ship_model.exp_now + gain;
		let (ship_level, next_exp) = level::exp_to_ship_level(new_ship_exp);
		let current_level_exp = level::ship_level_required_exp(ship_level);
		let progress = if next_exp > current_level_exp {
			((new_ship_exp - current_level_exp) * 100 / (next_exp - current_level_exp)).clamp(0, 99)
		} else {
			0
		};
		api_ship.api_lv = ship_level;
		api_ship.api_exp = [new_ship_exp, next_exp, progress];
		update_ship_impl(c, codex, &api_ship).await?;
	}

	snapshot.member_lv = updated_profile.hq_level;
	snapshot.member_exp = updated_profile.experience;
	Ok(snapshot)
}

async fn apply_sortie_map_result<C>(
	c: &C,
	profile_id: i64,
	definition: &MapDefinition,
	is_boss_cell: bool,
	snapshot: &SortieBattleResultSnapshot,
) -> Result<i64, GameplayError>
where
	C: ConnectionTrait,
{
	if !is_boss_cell || !matches!(snapshot.win_rank.as_str(), "S" | "A" | "B") {
		return Ok(0);
	}

	let record = find_map_record_impl(c, profile_id, definition.map_id).await?;
	let now = Utc::now();
	let was_cleared = record.cleared;
	let current_hp = record.current_hp;
	let previous_defeat_count = record.defeat_count.unwrap_or_default();
	let mut am = record.into_active_model();

	if let Some(max_hp) = definition.max_hp {
		let next_hp = (current_hp.unwrap_or(max_hp) - 1).max(0);
		let cleared = next_hp <= 0;
		am.current_hp = ActiveValue::Set(Some(next_hp));
		am.event_state = ActiveValue::Set(Some(if cleared {
			2
		} else {
			1
		}));
		if cleared {
			am.cleared = ActiveValue::Set(true);
			am.last_cleared_at = ActiveValue::Set(Some(now));
		}
		am.update(c).await?;
		return Ok(i64::from(!was_cleared && cleared));
	}

	if let Some(required) = definition.required_defeat_count {
		let next_defeat = previous_defeat_count + 1;
		let cleared = next_defeat >= required;
		am.defeat_count = ActiveValue::Set(Some(next_defeat.min(required)));
		if cleared {
			am.cleared = ActiveValue::Set(true);
			am.last_cleared_at = ActiveValue::Set(Some(now));
		}
		am.update(c).await?;
		return Ok(i64::from(!was_cleared && cleared));
	}

	if !was_cleared {
		am.cleared = ActiveValue::Set(true);
		am.last_cleared_at = ActiveValue::Set(Some(now));
		am.update(c).await?;
		return Ok(1);
	}

	am.update(c).await?;
	Ok(0)
}
