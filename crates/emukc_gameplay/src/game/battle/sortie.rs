#![allow(dead_code)]

use std::{
	collections::HashMap,
	sync::{LazyLock, Mutex},
};

use emukc_model::codex::Codex;

use super::core::{
	BattleContext, BattleOutcome, BattlePacket, BattleSimulation, simulate_day_battle_v1,
};

static PENDING_SORTIE_BATTLES: LazyLock<Mutex<HashMap<i64, SortieBattleSession>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct SortieBattleInput {
	pub profile_id: i64,
	pub deck_id: i64,
	pub map_id: i64,
	pub cell_id: i64,
	pub context: BattleContext,
}

#[derive(Debug, Clone)]
pub struct SortieBattleSession {
	pub profile_id: i64,
	pub deck_id: i64,
	pub map_id: i64,
	pub cell_id: i64,
	pub friendly_ship_ids: Vec<i64>,
	pub enemy_ship_ids: Vec<i64>,
	pub packet: BattlePacket,
	pub outcome: BattleOutcome,
}

pub fn simulate_and_store_sortie_day_battle(
	codex: &Codex,
	input: SortieBattleInput,
) -> SortieBattleSession {
	let SortieBattleInput {
		profile_id,
		deck_id,
		map_id,
		cell_id,
		context,
	} = input;
	let simulation = simulate_day_battle_v1(codex, context);
	let session = build_sortie_session(profile_id, deck_id, map_id, cell_id, simulation);
	PENDING_SORTIE_BATTLES.lock().unwrap().insert(session.profile_id, session.clone());
	session
}

pub fn take_sortie_day_battle_result(profile_id: i64) -> Option<SortieBattleSession> {
	PENDING_SORTIE_BATTLES.lock().unwrap().remove(&profile_id)
}

fn build_sortie_session(
	profile_id: i64,
	deck_id: i64,
	map_id: i64,
	cell_id: i64,
	simulation: BattleSimulation,
) -> SortieBattleSession {
	SortieBattleSession {
		profile_id,
		deck_id,
		map_id,
		cell_id,
		friendly_ship_ids: simulation.friendly.iter().map(|ship| ship.ship.api_id).collect(),
		enemy_ship_ids: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		packet: simulation.packet,
		outcome: simulation.outcome,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::game::battle::core::{BattleMode, BattleShipInput, EngagementType};
	use emukc_model::kc2::level;

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

	#[test]
	fn sortie_session_is_stored_until_result_is_taken() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let session = simulate_and_store_sortie_day_battle(
			&codex,
			SortieBattleInput {
				profile_id: 42,
				deck_id: 1,
				map_id: 11,
				cell_id: 3,
				context: BattleContext {
					mode: BattleMode::Sortie,
					friendly_formation_id: 1,
					enemy_formation_id: 1,
					engagement: EngagementType::SameCourse,
					friend_ships: vec![sample_ship(&codex, 89, 99)],
					enemy_ships: vec![sample_ship(&codex, 412, 99)],
				},
			},
		);

		assert_eq!(session.profile_id, 42);
		assert_eq!(session.map_id, 11);
		assert!(!session.enemy_ship_ids.is_empty());

		let taken = take_sortie_day_battle_result(42).unwrap();
		assert_eq!(taken.cell_id, 3);
		assert!(take_sortie_day_battle_result(42).is_none());
	}
}
