#![allow(non_snake_case)]

use serde::Serialize;

use emukc_model::{
	codex::Codex,
	kc2::{
		KcApiShip, KcApiSlotItem, KcSlotItemType3, KcSortieResultRank, level,
		start2::ApiMstSlotitem,
	},
	profile::practice::Rival,
};

use crate::err::GameplayError;

#[derive(Debug, Clone)]
pub struct PracticeBattleShipInput {
	pub ship: KcApiShip,
	pub slot_items: Vec<KcApiSlotItem>,
	pub effect_list: Vec<i64>,
}

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
	pub api_kouku: Option<PracticeKouku>,
	pub api_opening_taisen_flag: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_opening_taisen: Option<PracticeHougeki>,
	pub api_opening_flag: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_opening_atack: Option<PracticeOpeningAttack>,
	pub api_hourai_flag: [i64; 4],
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki1: Option<PracticeHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki2: Option<PracticeHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_hougeki3: Option<PracticeHougeki>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raigeki: Option<PracticeRaigeki>,
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

#[derive(Debug, Clone, Serialize)]
pub struct PracticeKouku {
	pub api_plane_from: [Vec<i64>; 2],
	pub api_stage1: PracticeKoukuStage1,
	pub api_stage2: PracticeKoukuStage2,
	pub api_stage3: PracticeKoukuStage3,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeKoukuStage1 {
	pub api_f_count: i64,
	pub api_f_lostcount: i64,
	pub api_e_count: i64,
	pub api_e_lostcount: i64,
	pub api_disp_seiku: i64,
	pub api_touch_plane: [i64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeKoukuStage2 {
	pub api_f_count: i64,
	pub api_f_lostcount: i64,
	pub api_e_count: i64,
	pub api_e_lostcount: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeKoukuStage3 {
	pub api_frai_flag: Vec<i64>,
	pub api_erai_flag: Vec<i64>,
	pub api_fbak_flag: Vec<i64>,
	pub api_ebak_flag: Vec<i64>,
	pub api_fcl_flag: Vec<i64>,
	pub api_ecl_flag: Vec<i64>,
	pub api_fdam: Vec<i64>,
	pub api_edam: Vec<i64>,
	pub api_f_sp_list: Vec<Option<i64>>,
	pub api_e_sp_list: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeOpeningAttack {
	pub api_frai_list_items: Vec<Option<Vec<i64>>>,
	pub api_fcl_list_items: Vec<Option<Vec<i64>>>,
	pub api_fdam: Vec<i64>,
	pub api_fydam_list_items: Vec<Option<Vec<i64>>>,
	pub api_erai_list_items: Vec<Option<Vec<i64>>>,
	pub api_ecl_list_items: Vec<Option<Vec<i64>>>,
	pub api_edam: Vec<i64>,
	pub api_eydam_list_items: Vec<Option<Vec<i64>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeHougeki {
	pub api_at_eflag: Vec<i64>,
	pub api_at_list: Vec<i64>,
	pub api_at_type: Vec<i64>,
	pub api_df_list: Vec<Vec<i64>>,
	pub api_si_list: Vec<Vec<i64>>,
	pub api_cl_list: Vec<Vec<i64>>,
	pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeRaigeki {
	pub api_frai: Vec<i64>,
	pub api_fcl: Vec<i64>,
	pub api_fdam: Vec<i64>,
	pub api_fydam: Vec<i64>,
	pub api_erai: Vec<i64>,
	pub api_ecl: Vec<i64>,
	pub api_edam: Vec<i64>,
	pub api_eydam: Vec<i64>,
}

#[derive(Debug, Clone)]
struct RuntimeShip {
	ship: KcApiShip,
	slot_items: Vec<KcApiSlotItem>,
	effect_list: Vec<i64>,
	current_hp: i64,
	damage_dealt: i64,
}

pub fn simulate_practice_day_battle(
	codex: &Codex,
	input: PracticeBattleInput,
) -> Result<(PracticeBattleResponse, PracticeBattleResultSnapshot), GameplayError> {
	let mut friendly = input
		.friend_ships
		.into_iter()
		.map(|ship| RuntimeShip {
			current_hp: ship.ship.api_nowhp,
			damage_dealt: 0,
			ship: ship.ship,
			slot_items: ship.slot_items,
			effect_list: ship.effect_list,
		})
		.collect::<Vec<_>>();
	let mut enemy = input
		.enemy_ships
		.into_iter()
		.map(|ship| RuntimeShip {
			current_hp: ship.ship.api_nowhp,
			damage_dealt: 0,
			ship: ship.ship,
			slot_items: ship.slot_items,
			effect_list: ship.effect_list,
		})
		.collect::<Vec<_>>();

	let formation = [input.formation_id, 1, 1];
	let mut opening_atack = None;
	let mut hougeki1 = None;
	let mut hougeki2 = None;
	let hougeki3 = None;
	let mut raigeki = None;
	let mut kouku = None;
	let mut stage_flag = [0, 0, 0];
	let mut hourai_flag = [0, 0, 0, 0];

	if has_any_aircraft(codex, &friendly) || has_any_aircraft(codex, &enemy) {
		stage_flag = [1, 1, 1];
		kouku = Some(simulate_kouku(codex, &mut friendly, &mut enemy));
	}

	if can_opening_torpedo(&friendly) || can_opening_torpedo(&enemy) {
		let opening = simulate_opening_torpedo(&mut friendly, &mut enemy);
		if opening.is_some() {
			opening_atack = opening;
			hourai_flag = [1, 0, 0, 0];
		}
	}

	let shelling_round_1 = simulate_shelling_round(&friendly, &enemy, false);
	if let Some(round) = resolve_shelling_round(codex, &mut friendly, &mut enemy, shelling_round_1)
	{
		hougeki1 = Some(round);
		hourai_flag[0] = 1;
	}

	if any_alive(&friendly) && any_alive(&enemy) {
		let shelling_round_2 = simulate_shelling_round(&enemy, &friendly, true);
		if let Some(round) =
			resolve_shelling_round(codex, &mut friendly, &mut enemy, shelling_round_2)
		{
			hougeki2 = Some(round);
			hourai_flag[1] = 1;
		}
	}

	if any_alive(&friendly) && any_alive(&enemy) && (can_torpedo(&friendly) || can_torpedo(&enemy))
	{
		if let Some(round) = simulate_raigeki(&mut friendly, &mut enemy) {
			raigeki = Some(round);
			hourai_flag[3] = 1;
		}
	}

	let base_exp = calculate_practice_base_exp(&input.rival);
	let win_rank = calculate_win_rank(&friendly, &enemy);
	let mvp_idx = calculate_mvp(&friendly);
	let get_exp = calculate_admiral_exp(base_exp, &win_rank);
	let (ship_exp, ship_lvup) = calculate_practice_ship_exp(&friendly, base_exp, mvp_idx);

	let response = PracticeBattleResponse {
		api_deck_id: input.deck_id,
		api_formation: formation,
		api_f_nowhps: friendly.iter().map(|ship| ship.current_hp.max(0)).collect(),
		api_f_maxhps: friendly.iter().map(|ship| ship.ship.api_maxhp).collect(),
		api_fParam: friendly
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
		api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
		api_e_nowhps: enemy.iter().map(|ship| ship.current_hp.max(0)).collect(),
		api_e_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
		api_eSlot: enemy.iter().map(|ship| enemy_slot_ids(ship)).collect(),
		api_eParam: enemy
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
		api_e_effect_list: enemy
			.iter()
			.map(|ship| {
				if ship.effect_list.is_empty() {
					vec![0]
				} else {
					ship.effect_list.clone()
				}
			})
			.collect(),
		api_smoke_type: 0,
		api_balloon_cell: 0,
		api_atoll_cell: 0,
		api_midnight_flag: 0,
		api_search: [1, 1],
		api_stage_flag: stage_flag,
		api_kouku: kouku,
		api_opening_taisen_flag: 0,
		api_opening_taisen: None,
		api_opening_flag: if opening_atack.is_some() {
			1
		} else {
			0
		},
		api_opening_atack: opening_atack,
		api_hourai_flag: hourai_flag,
		api_hougeki1: hougeki1,
		api_hougeki2: hougeki2,
		api_hougeki3: hougeki3,
		api_raigeki: raigeki,
	};

	let snapshot = PracticeBattleResultSnapshot {
		enemy_id: input.enemy_id,
		enemy_ship_ids: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
		win_rank,
		get_exp,
		member_lv: input.member_lv,
		member_exp: input.member_exp,
		get_base_exp: base_exp,
		mvp: mvp_idx,
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

#[derive(Debug, Clone)]
struct PendingShell {
	attacker_enemy: bool,
	attacker_idx: i64,
	defender_idx: i64,
	weapon_ids: Vec<i64>,
	damage: i64,
}

fn simulate_shelling_round(
	attackers: &[RuntimeShip],
	defenders: &[RuntimeShip],
	attacker_enemy: bool,
) -> Vec<PendingShell> {
	let Some(first_alive_defender) = defenders.iter().position(|ship| ship.current_hp > 0) else {
		return vec![];
	};

	attackers
		.iter()
		.enumerate()
		.filter(|(_, ship)| ship.current_hp > 0)
		.map(|(idx, ship)| PendingShell {
			attacker_enemy,
			attacker_idx: idx as i64,
			defender_idx: first_alive_defender as i64,
			weapon_ids: weapon_display_ids(ship),
			damage: 0.max(ship.ship.api_karyoku[0] + weapon_attack_bonus(ship) - 30) + 1,
		})
		.collect()
}

fn resolve_shelling_round(
	codex: &Codex,
	friendly: &mut [RuntimeShip],
	enemy: &mut [RuntimeShip],
	round: Vec<PendingShell>,
) -> Option<PracticeHougeki> {
	if round.is_empty() {
		return None;
	}

	let mut at_eflag = Vec::with_capacity(round.len());
	let mut at_list = Vec::with_capacity(round.len());
	let mut at_type = Vec::with_capacity(round.len());
	let mut df_list = Vec::with_capacity(round.len());
	let mut si_list = Vec::with_capacity(round.len());
	let mut cl_list = Vec::with_capacity(round.len());
	let mut damage = Vec::with_capacity(round.len());

	for action in round {
		let attacker_enemy = action.attacker_enemy;
		let target = if attacker_enemy {
			friendly.get_mut(action.defender_idx as usize)?
		} else {
			enemy.get_mut(action.defender_idx as usize)?
		};
		let dealt = action.damage.min(target.current_hp.max(0));
		target.current_hp -= dealt;

		at_eflag.push(if attacker_enemy {
			1
		} else {
			0
		});
		at_list.push(action.attacker_idx);
		at_type.push(0);
		df_list.push(vec![action.defender_idx]);
		si_list.push(action.weapon_ids);
		cl_list.push(vec![1]);
		damage.push(vec![dealt]);

		if !attacker_enemy {
			if let Some(attacker) = friendly.get_mut(action.attacker_idx as usize) {
				attacker.damage_dealt += dealt;
			}
		}
	}

	let _ = codex;
	Some(PracticeHougeki {
		api_at_eflag: at_eflag,
		api_at_list: at_list,
		api_at_type: at_type,
		api_df_list: df_list,
		api_si_list: si_list,
		api_cl_list: cl_list,
		api_damage: damage,
	})
}

fn simulate_opening_torpedo(
	friendly: &mut [RuntimeShip],
	enemy: &mut [RuntimeShip],
) -> Option<PracticeOpeningAttack> {
	let len = 7;
	let mut api_frai_list_items = vec![None; len];
	let mut api_fcl_list_items = vec![None; len];
	let mut api_fdam = vec![0; len];
	let mut api_fydam_list_items = vec![None; len];
	let mut api_erai_list_items = vec![None; len];
	let mut api_ecl_list_items = vec![None; len];
	let mut api_edam = vec![0; len];
	let mut api_eydam_list_items = vec![None; len];
	let mut happened = false;

	if let Some(target_idx) = enemy.iter().position(|ship| ship.current_hp > 0) {
		for (idx, ship) in friendly.iter_mut().enumerate() {
			if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
				continue;
			}
			let dealt = (ship.ship.api_raisou[0] / 4).max(1);
			let dealt = dealt.min(enemy[target_idx].current_hp.max(0));
			enemy[target_idx].current_hp -= dealt;
			ship.damage_dealt += dealt;
			api_frai_list_items[idx] = Some(vec![target_idx as i64]);
			api_fcl_list_items[idx] = Some(vec![1]);
			api_eydam_list_items[idx] = Some(vec![dealt]);
			api_edam[target_idx] += dealt;
			happened = true;
		}
	}

	if let Some(target_idx) = friendly.iter().position(|ship| ship.current_hp > 0) {
		for (idx, ship) in enemy.iter_mut().enumerate() {
			if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
				continue;
			}
			let dealt = (ship.ship.api_raisou[0] / 4).max(1);
			let dealt = dealt.min(friendly[target_idx].current_hp.max(0));
			friendly[target_idx].current_hp -= dealt;
			api_erai_list_items[idx] = Some(vec![target_idx as i64]);
			api_ecl_list_items[idx] = Some(vec![1]);
			api_fydam_list_items[idx] = Some(vec![dealt]);
			api_fdam[target_idx] += dealt;
			happened = true;
		}
	}

	happened.then_some(PracticeOpeningAttack {
		api_frai_list_items,
		api_fcl_list_items,
		api_fdam,
		api_fydam_list_items,
		api_erai_list_items,
		api_ecl_list_items,
		api_edam,
		api_eydam_list_items,
	})
}

fn simulate_raigeki(
	friendly: &mut [RuntimeShip],
	enemy: &mut [RuntimeShip],
) -> Option<PracticeRaigeki> {
	let len = 7;
	let mut api_frai = vec![-1; len];
	let mut api_fcl = vec![0; len];
	let mut api_fdam = vec![0; len];
	let mut api_fydam = vec![0; len];
	let mut api_erai = vec![-1; len];
	let mut api_ecl = vec![0; len];
	let mut api_edam = vec![0; len];
	let mut api_eydam = vec![0; len];
	let mut happened = false;

	for (idx, ship) in friendly.iter_mut().enumerate() {
		if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
			continue;
		}
		let Some(target_idx) = enemy.iter().position(|enemy| enemy.current_hp > 0) else {
			break;
		};
		let dealt = (ship.ship.api_raisou[0] / 3).max(1).min(enemy[target_idx].current_hp.max(0));
		enemy[target_idx].current_hp -= dealt;
		ship.damage_dealt += dealt;
		api_frai[idx] = target_idx as i64;
		api_fcl[idx] = 1;
		api_eydam[idx] = dealt;
		api_edam[target_idx] += dealt;
		happened = true;
	}

	for (idx, ship) in enemy.iter_mut().enumerate() {
		if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
			continue;
		}
		let Some(target_idx) = friendly.iter().position(|friend| friend.current_hp > 0) else {
			break;
		};
		let dealt =
			(ship.ship.api_raisou[0] / 3).max(1).min(friendly[target_idx].current_hp.max(0));
		friendly[target_idx].current_hp -= dealt;
		api_erai[idx] = target_idx as i64;
		api_ecl[idx] = 1;
		api_fydam[idx] = dealt;
		api_fdam[target_idx] += dealt;
		happened = true;
	}

	happened.then_some(PracticeRaigeki {
		api_frai,
		api_fcl,
		api_fdam,
		api_fydam,
		api_erai,
		api_ecl,
		api_edam,
		api_eydam,
	})
}

fn simulate_kouku(
	codex: &Codex,
	friendly: &mut [RuntimeShip],
	enemy: &mut [RuntimeShip],
) -> PracticeKouku {
	let friend_planes = total_plane_count(codex, friendly);
	let enemy_planes = total_plane_count(codex, enemy);
	let mut api_edam = vec![0; enemy.len()];
	let mut api_fdam = vec![0; friendly.len()];

	if friend_planes > 0 {
		if let Some(target_idx) = enemy.iter().position(|ship| ship.current_hp > 0) {
			let dealt = 6.min(enemy[target_idx].current_hp.max(0));
			enemy[target_idx].current_hp -= dealt;
			api_edam[target_idx] = dealt;
		}
	}
	if enemy_planes > 0 {
		if let Some(target_idx) = friendly.iter().position(|ship| ship.current_hp > 0) {
			let dealt = 3.min(friendly[target_idx].current_hp.max(0));
			friendly[target_idx].current_hp -= dealt;
			api_fdam[target_idx] = dealt;
		}
	}

	PracticeKouku {
		api_plane_from: [plane_from(codex, friendly), plane_from(codex, enemy)],
		api_stage1: PracticeKoukuStage1 {
			api_f_count: friend_planes,
			api_f_lostcount: 0,
			api_e_count: enemy_planes,
			api_e_lostcount: 0,
			api_disp_seiku: if friend_planes >= enemy_planes {
				1
			} else {
				0
			},
			api_touch_plane: [
				first_touch_plane(codex, friendly).unwrap_or(-1),
				first_touch_plane(codex, enemy).unwrap_or(-1),
			],
		},
		api_stage2: PracticeKoukuStage2 {
			api_f_count: friend_planes,
			api_f_lostcount: friend_planes.min(4),
			api_e_count: enemy_planes,
			api_e_lostcount: enemy_planes.min(enemy_planes),
		},
		api_stage3: PracticeKoukuStage3 {
			api_frai_flag: vec![0; friendly.len()],
			api_erai_flag: api_edam.iter().map(|dam| i64::from(*dam > 0)).collect(),
			api_fbak_flag: vec![0; friendly.len()],
			api_ebak_flag: api_edam.iter().map(|dam| i64::from(*dam > 0)).collect(),
			api_fcl_flag: api_fdam.iter().map(|dam| i64::from(*dam > 0)).collect(),
			api_ecl_flag: api_edam.iter().map(|dam| i64::from(*dam > 0)).collect(),
			api_fdam,
			api_edam,
			api_f_sp_list: vec![None; friendly.len()],
			api_e_sp_list: vec![None; enemy.len()],
		},
	}
}

fn total_plane_count(codex: &Codex, ships: &[RuntimeShip]) -> i64 {
	ships
		.iter()
		.flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
		.filter(|(slot_item, onslot)| {
			*onslot > 0
				&& codex
					.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
					.ok()
					.is_some_and(|mst| is_aircraft_type(mst.api_type[2]))
		})
		.map(|(_, onslot)| onslot)
		.sum()
}

fn has_any_aircraft(codex: &Codex, ships: &[RuntimeShip]) -> bool {
	total_plane_count(codex, ships) > 0
}

fn plane_from(codex: &Codex, ships: &[RuntimeShip]) -> Vec<i64> {
	ships
		.iter()
		.enumerate()
		.filter_map(|(idx, ship)| {
			let has_plane =
				ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
					onslot > 0
						&& codex
							.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
							.ok()
							.is_some_and(|mst| is_aircraft_type(mst.api_type[2]))
				});
			has_plane.then_some(idx as i64 + 1)
		})
		.collect()
}

fn first_touch_plane(codex: &Codex, ships: &[RuntimeShip]) -> Option<i64> {
	ships.iter().flat_map(|ship| ship.slot_items.iter()).find_map(|slot_item| {
		codex
			.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
			.ok()
			.filter(|mst| {
				matches!(
					KcSlotItemType3::n(mst.api_type[2]),
					Some(KcSlotItemType3::CarrierBasedRecon | KcSlotItemType3::CarrierBasedRecon2)
				)
			})
			.map(|mst| mst.api_id)
	})
}

fn weapon_display_ids(ship: &RuntimeShip) -> Vec<i64> {
	let ids = ship
		.slot_items
		.iter()
		.map(|slot_item| slot_item.api_slotitem_id)
		.take(2)
		.collect::<Vec<_>>();
	if ids.is_empty() {
		vec![-1]
	} else {
		ids
	}
}

fn weapon_attack_bonus(ship: &RuntimeShip) -> i64 {
	ship.slot_items.len() as i64 * 5
}

fn can_opening_torpedo(ships: &[RuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0 && ship.ship.api_raisou[0] > 0)
}

fn can_torpedo(ships: &[RuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0 && ship.ship.api_raisou[0] > 0)
}

fn any_alive(ships: &[RuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0)
}

fn enemy_slot_ids(ship: &RuntimeShip) -> [i64; 5] {
	let mut slots = [-1; 5];
	for (idx, slot_item) in ship.slot_items.iter().take(5).enumerate() {
		slots[idx] = slot_item.api_slotitem_id;
	}
	slots
}

fn is_aircraft_type(slotitem_type: i64) -> bool {
	matches!(
		KcSlotItemType3::n(slotitem_type),
		Some(
			KcSlotItemType3::CarrierBasedFighter
				| KcSlotItemType3::CarrierBasedDiveBomber
				| KcSlotItemType3::CarrierBasedTorpedoBomber
				| KcSlotItemType3::CarrierBasedRecon
				| KcSlotItemType3::CarrierBasedRecon2
		)
	)
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
	friendly: &[RuntimeShip],
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

fn calculate_mvp(friendly: &[RuntimeShip]) -> i64 {
	friendly
		.iter()
		.enumerate()
		.max_by_key(|(_, ship)| ship.damage_dealt)
		.map(|(idx, _)| idx as i64 + 1)
		.unwrap_or(-1)
}

fn calculate_win_rank(friendly: &[RuntimeShip], enemy: &[RuntimeShip]) -> String {
	let enemy_total_hp: i64 = enemy.iter().map(|ship| ship.ship.api_maxhp).sum();
	let enemy_remaining_hp: i64 = enemy.iter().map(|ship| ship.current_hp.max(0)).sum();
	let friend_total_hp: i64 = friendly.iter().map(|ship| ship.ship.api_maxhp).sum();
	let friend_remaining_hp: i64 = friendly.iter().map(|ship| ship.current_hp.max(0)).sum();
	let enemy_sunk = enemy.iter().all(|ship| ship.current_hp <= 0);
	let friend_lost = friendly.iter().all(|ship| ship.current_hp <= 0);
	let enemy_damage_rate =
		(enemy_total_hp - enemy_remaining_hp) as f64 / enemy_total_hp.max(1) as f64;
	let friend_damage_rate =
		(friend_total_hp - friend_remaining_hp) as f64 / friend_total_hp.max(1) as f64;

	let rank = if enemy_sunk {
		KcSortieResultRank::S
	} else if enemy_damage_rate >= 0.7 {
		KcSortieResultRank::A
	} else if enemy_damage_rate > friend_damage_rate {
		KcSortieResultRank::B
	} else if !friend_lost {
		KcSortieResultRank::C
	} else {
		KcSortieResultRank::D
	};

	match rank {
		KcSortieResultRank::S => "S",
		KcSortieResultRank::A => "A",
		KcSortieResultRank::B => "B",
		KcSortieResultRank::C => "C",
		KcSortieResultRank::D => "D",
		KcSortieResultRank::E => "E",
	}
	.to_string()
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
}
