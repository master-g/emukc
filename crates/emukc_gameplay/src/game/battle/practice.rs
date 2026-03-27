#![allow(non_snake_case)]

use serde::Serialize;

use emukc_model::{codex::Codex, kc2::level, profile::practice::Rival};

use crate::{
	err::GameplayError,
	game::battle::core::{
		BattleContext, BattleHougeki, BattleKouku, BattleMode, BattleOpeningAttack, BattleRaigeki,
		BattleRuntimeShip, BattleShipInput, EngagementType, simulate_day_battle_v1,
	},
};

pub type PracticeBattleShipInput = BattleShipInput;

#[derive(Debug, Clone)]
pub struct PracticeBattleInput {
	pub deck_id: i64,
	pub formation_id: i64,
	pub enemy_id: i64,
	pub member_lv: i64,
	pub member_exp: i64,
	pub friend_ships: Vec<PracticeBattleShipInput>,
	pub enemy_ships: Vec<PracticeBattleShipInput>,
	pub rival: Rival,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResponse {
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
	pub api_e_effect_list: Vec<Vec<i64>>,
	pub api_smoke_type: i64,
	pub api_balloon_cell: i64,
	pub api_atoll_cell: i64,
	pub api_midnight_flag: i64,
	pub api_search: [i64; 2],
	pub api_stage_flag: [i64; 3],
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kouku: Option<BattleKouku>,
	pub api_opening_taisen_flag: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_opening_taisen: Option<BattleHougeki>,
	pub api_opening_flag: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_opening_atack: Option<BattleOpeningAttack>,
	pub api_hourai_flag: [i64; 4],
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki1: Option<BattleHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki2: Option<BattleHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki3: Option<BattleHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raigeki: Option<BattleRaigeki>,
}

#[derive(Debug, Clone)]
pub struct PracticeBattleResultSnapshot {
	pub enemy_id: i64,
	pub enemy_ship_ids: Vec<i64>,
	pub win_rank: String,
	pub get_exp: i64,
	pub member_lv: i64,
	pub member_exp: i64,
	pub get_base_exp: i64,
	pub mvp: i64,
	pub get_ship_exp: Vec<i64>,
	pub get_exp_lvup: Vec<Vec<i64>>,
	pub enemy_level: i64,
	pub enemy_rank: String,
	pub enemy_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResultResponse {
	pub api_ship_id: Vec<i64>,
	pub api_win_rank: String,
	pub api_get_exp: i64,
	pub api_member_lv: i64,
	pub api_member_exp: i64,
	pub api_get_base_exp: i64,
	pub api_mvp: i64,
	pub api_get_ship_exp: Vec<i64>,
	pub api_get_exp_lvup: Vec<Vec<i64>>,
	pub api_enemy_info: PracticeBattleEnemyInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleEnemyInfo {
	pub api_user_name: String,
	pub api_level: i64,
	pub api_rank: String,
	pub api_deck_name: String,
}

pub fn simulate_practice_day_battle(
	codex: &Codex,
	input: PracticeBattleInput,
) -> Result<(PracticeBattleResponse, PracticeBattleResultSnapshot), GameplayError> {
	let simulation = simulate_day_battle_v1(
		codex,
		BattleContext {
			mode: BattleMode::Practice,
			friendly_formation_id: input.formation_id,
			enemy_formation_id: 1,
			engagement: EngagementType::SameCourse,
			friend_ships: input.friend_ships,
			enemy_ships: input.enemy_ships,
		},
	);

	let base_exp = calculate_practice_base_exp(&input.rival);
	let get_exp = calculate_admiral_exp(base_exp, &simulation.outcome.win_rank);
	let (ship_exp, ship_lvup) =
		calculate_practice_ship_exp(&simulation.friendly, base_exp, simulation.outcome.mvp);

	let response = PracticeBattleResponse {
		api_deck_id: input.deck_id,
		api_formation: simulation.packet.formation,
		api_f_nowhps: simulation.packet.friendly_nowhps,
		api_f_maxhps: simulation.packet.friendly_maxhps,
		api_fParam: simulation
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
		api_ship_ke: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		api_ship_lv: simulation.enemy.iter().map(|ship| ship.ship.api_lv).collect(),
		api_e_nowhps: simulation.packet.enemy_nowhps,
		api_e_maxhps: simulation.packet.enemy_maxhps,
		api_eSlot: simulation.enemy.iter().map(enemy_slot_ids).collect(),
		api_eParam: simulation
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
		api_e_effect_list: simulation
			.enemy
			.iter()
			.map(|ship| {
				if ship.effect_list.is_empty() {
					vec![0]
				} else {
					ship.effect_list.clone()
				}
			})
			.collect(),
		api_smoke_type: simulation.packet.smoke_type,
		api_balloon_cell: simulation.packet.balloon_cell,
		api_atoll_cell: simulation.packet.atoll_cell,
		api_midnight_flag: simulation.packet.midnight_flag,
		api_search: simulation.packet.search,
		api_stage_flag: simulation.packet.stage_flag,
		api_kouku: simulation.packet.kouku,
		api_opening_taisen_flag: simulation.packet.opening_taisen_flag,
		api_opening_taisen: simulation.packet.opening_taisen,
		api_opening_flag: simulation.packet.opening_flag,
		api_opening_atack: simulation.packet.opening_attack,
		api_hourai_flag: simulation.packet.hourai_flag,
		api_hougeki1: simulation.packet.hougeki1,
		api_hougeki2: simulation.packet.hougeki2,
		api_hougeki3: simulation.packet.hougeki3,
		api_raigeki: simulation.packet.raigeki,
	};

	let snapshot = PracticeBattleResultSnapshot {
		enemy_id: input.enemy_id,
		enemy_ship_ids: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		win_rank: simulation.outcome.win_rank,
		get_exp,
		member_lv: input.member_lv,
		member_exp: input.member_exp,
		get_base_exp: base_exp,
		mvp: simulation.outcome.mvp,
		get_ship_exp: ship_exp,
		get_exp_lvup: ship_lvup,
		enemy_level: input.rival.level,
		enemy_rank: input.rival.rank.get_name().to_string(),
		enemy_deck_name: input.rival.details.deck_name,
	};

	Ok((response, snapshot))
}

pub fn build_practice_battle_result_response(
	snapshot: PracticeBattleResultSnapshot,
) -> PracticeBattleResultResponse {
	PracticeBattleResultResponse {
		api_ship_id: snapshot.enemy_ship_ids,
		api_win_rank: snapshot.win_rank,
		api_get_exp: snapshot.get_exp,
		api_member_lv: snapshot.member_lv,
		api_member_exp: snapshot.member_exp,
		api_get_base_exp: snapshot.get_base_exp,
		api_mvp: snapshot.mvp,
		api_get_ship_exp: snapshot.get_ship_exp,
		api_get_exp_lvup: snapshot.get_exp_lvup,
		api_enemy_info: PracticeBattleEnemyInfo {
			api_user_name: String::new(),
			api_level: snapshot.enemy_level,
			api_rank: snapshot.enemy_rank,
			api_deck_name: snapshot.enemy_deck_name,
		},
	}
}

fn enemy_slot_ids(ship: &BattleRuntimeShip) -> [i64; 5] {
	let mut slots = [-1; 5];
	for (idx, slot_item) in ship.slot_items.iter().take(5).enumerate() {
		slots[idx] = slot_item.api_slotitem_id;
	}
	slots
}

fn calculate_practice_base_exp(rival: &Rival) -> i64 {
	(rival.level.max(1) * 9).clamp(100, 1200)
}

fn calculate_admiral_exp(base_exp: i64, win_rank: &str) -> i64 {
	match win_rank {
		"S" => (base_exp as f64 * 0.12).round() as i64,
		"A" => (base_exp as f64 * 0.1).round() as i64,
		"B" => (base_exp as f64 * 0.08).round() as i64,
		"C" => (base_exp as f64 * 0.05).round() as i64,
		_ => (base_exp as f64 * 0.03).round() as i64,
	}
}

fn calculate_practice_ship_exp(
	friendly: &[BattleRuntimeShip],
	base_exp: i64,
	mvp_idx: i64,
) -> (Vec<i64>, Vec<Vec<i64>>) {
	let mut exp = vec![-1];
	let mut lvup = Vec::with_capacity(friendly.len());

	for (idx, ship) in friendly.iter().enumerate() {
		let gain = if idx as i64 + 1 == mvp_idx {
			base_exp * 2
		} else if idx == 0 {
			((base_exp as f64) * 1.5).floor() as i64
		} else {
			base_exp
		};
		exp.push(gain);

		let new_exp = ship.ship.api_exp[0] + gain;
		let (_, next_exp) = level::exp_to_ship_level(new_exp);
		lvup.push(vec![ship.ship.api_exp[0], next_exp]);
	}

	(exp, lvup)
}

#[cfg(test)]
mod tests {
	use super::*;
	use emukc_model::codex::Codex;

	fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> PracticeBattleShipInput {
		let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
		let exp_now = level::ship_level_required_exp(level);
		let (_, next_exp) = level::exp_to_ship_level(exp_now);
		ship.api_lv = level;
		ship.api_exp = [exp_now, next_exp, 0];
		codex.cal_ship_status(&mut ship, &slot_items).unwrap();
		PracticeBattleShipInput {
			ship,
			slot_items,
			effect_list: vec![0],
		}
	}

	#[test]
	fn practice_battle_core_generates_packet_and_result() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let input = PracticeBattleInput {
			deck_id: 1,
			formation_id: 1,
			enemy_id: 1,
			member_lv: 120,
			member_exp: 123456,
			friend_ships: vec![sample_ship(&codex, 89, 99), sample_ship(&codex, 79, 80)],
			enemy_ships: vec![sample_ship(&codex, 412, 185)],
			rival: Rival {
				id: 1,
				index: 1,
				name: "Enemy".to_string(),
				comment: String::new(),
				level: 120,
				rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
				flag: emukc_model::profile::practice::RivalFlag::Gold,
				status: emukc_model::profile::practice::RivalStatus::Untouched,
				medals: 10,
				details: emukc_model::profile::practice::RivalDetail {
					deck_name: "演習".to_string(),
					..Default::default()
				},
			},
		};

		let (battle, result) = simulate_practice_day_battle(&codex, input).unwrap();
		assert_eq!(battle.api_deck_id, 1);
		assert_eq!(battle.api_formation, [1, 1, 1]);
		assert_eq!(battle.api_f_nowhps.len(), 2);
		assert_eq!(battle.api_ship_ke.len(), 1);
		assert_eq!(result.enemy_ship_ids.len(), 1);
		assert_eq!(result.member_lv, 120);
	}

	#[test]
	fn battle_midnight_flag_stays_disabled_until_night_battle_exists() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let input = PracticeBattleInput {
			deck_id: 1,
			formation_id: 5,
			enemy_id: 1,
			member_lv: 120,
			member_exp: 123456,
			friend_ships: vec![sample_ship(&codex, 79, 1)],
			enemy_ships: vec![sample_ship(&codex, 412, 99)],
			rival: Rival {
				id: 1,
				index: 1,
				name: "Enemy".to_string(),
				comment: String::new(),
				level: 120,
				rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
				flag: emukc_model::profile::practice::RivalFlag::Gold,
				status: emukc_model::profile::practice::RivalStatus::Untouched,
				medals: 10,
				details: emukc_model::profile::practice::RivalDetail {
					deck_name: "演習".to_string(),
					..Default::default()
				},
			},
		};

		let (battle, _) = simulate_practice_day_battle(&codex, input).unwrap();
		assert_eq!(battle.api_midnight_flag, 0);
	}
}
