use serde::Serialize;

use emukc_model::{
	codex::Codex,
	kc2::{KcApiShip, KcApiSlotItem, KcSlotItemType3, KcSortieResultRank, start2::ApiMstSlotitem},
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleMode {
	Practice,
	Sortie,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngagementType {
	SameCourse,
	HeadOn,
	TAdvantage,
	TDisadvantage,
}

impl EngagementType {
	pub const fn api_id(self) -> i64 {
		match self {
			Self::SameCourse => 1,
			Self::HeadOn => 2,
			Self::TAdvantage => 3,
			Self::TDisadvantage => 4,
		}
	}

	pub const fn modifier(self) -> f64 {
		match self {
			Self::SameCourse => 1.0,
			Self::HeadOn => 0.8,
			Self::TAdvantage => 1.2,
			Self::TDisadvantage => 0.6,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BattleShipInput {
	pub ship: KcApiShip,
	pub slot_items: Vec<KcApiSlotItem>,
	pub effect_list: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct BattleRuntimeShip {
	pub ship: KcApiShip,
	pub slot_items: Vec<KcApiSlotItem>,
	pub effect_list: Vec<i64>,
	pub current_hp: i64,
	pub damage_dealt: i64,
}

impl From<BattleShipInput> for BattleRuntimeShip {
	fn from(ship: BattleShipInput) -> Self {
		Self {
			current_hp: ship.ship.api_nowhp,
			damage_dealt: 0,
			ship: ship.ship,
			slot_items: ship.slot_items,
			effect_list: ship.effect_list,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BattleContext {
	pub mode: BattleMode,
	pub friendly_formation_id: i64,
	pub enemy_formation_id: i64,
	pub engagement: EngagementType,
	pub friend_ships: Vec<BattleShipInput>,
	pub enemy_ships: Vec<BattleShipInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKouku {
	pub api_plane_from: [Vec<i64>; 2],
	pub api_stage1: BattleKoukuStage1,
	pub api_stage2: BattleKoukuStage2,
	pub api_stage3: BattleKoukuStage3,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage1 {
	pub api_f_count: i64,
	pub api_f_lostcount: i64,
	pub api_e_count: i64,
	pub api_e_lostcount: i64,
	pub api_disp_seiku: i64,
	pub api_touch_plane: [i64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage2 {
	pub api_f_count: i64,
	pub api_f_lostcount: i64,
	pub api_e_count: i64,
	pub api_e_lostcount: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage3 {
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
pub struct BattleOpeningAttack {
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
pub struct BattleHougeki {
	pub api_at_eflag: Vec<i64>,
	pub api_at_list: Vec<i64>,
	pub api_at_type: Vec<i64>,
	pub api_df_list: Vec<Vec<i64>>,
	pub api_si_list: Vec<Vec<i64>>,
	pub api_cl_list: Vec<Vec<i64>>,
	pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleRaigeki {
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
pub struct BattlePacket {
	pub formation: [i64; 3],
	pub friendly_nowhps: Vec<i64>,
	pub friendly_maxhps: Vec<i64>,
	pub enemy_nowhps: Vec<i64>,
	pub enemy_maxhps: Vec<i64>,
	pub smoke_type: i64,
	pub balloon_cell: i64,
	pub atoll_cell: i64,
	pub midnight_flag: i64,
	pub search: [i64; 2],
	pub stage_flag: [i64; 3],
	pub kouku: Option<BattleKouku>,
	pub opening_taisen_flag: i64,
	pub opening_taisen: Option<BattleHougeki>,
	pub opening_flag: i64,
	pub opening_attack: Option<BattleOpeningAttack>,
	pub hourai_flag: [i64; 4],
	pub hougeki1: Option<BattleHougeki>,
	pub hougeki2: Option<BattleHougeki>,
	pub hougeki3: Option<BattleHougeki>,
	pub raigeki: Option<BattleRaigeki>,
}

#[derive(Debug, Clone)]
pub struct BattleOutcome {
	pub win_rank: String,
	pub mvp: i64,
	#[allow(dead_code)]
	pub can_midnight: bool,
}

#[derive(Debug, Clone)]
pub struct BattleSimulation {
	pub friendly: Vec<BattleRuntimeShip>,
	pub enemy: Vec<BattleRuntimeShip>,
	pub packet: BattlePacket,
	pub outcome: BattleOutcome,
}

#[derive(Debug, Clone)]
struct PendingShell {
	attacker_enemy: bool,
	attacker_idx: i64,
	defender_idx: i64,
	weapon_ids: Vec<i64>,
	damage: i64,
}

pub fn simulate_day_battle_v1(codex: &Codex, context: BattleContext) -> BattleSimulation {
	let mut friendly =
		context.friend_ships.into_iter().map(BattleRuntimeShip::from).collect::<Vec<_>>();
	let mut enemy =
		context.enemy_ships.into_iter().map(BattleRuntimeShip::from).collect::<Vec<_>>();

	let mut opening_attack = None;
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
		opening_attack = simulate_opening_torpedo(
			&mut friendly,
			&mut enemy,
			context.friendly_formation_id,
			context.enemy_formation_id,
			context.engagement,
		);
		if opening_attack.is_some() {
			hourai_flag[0] = 1;
		}
	}

	let shelling_round_1 = simulate_shelling_round(
		&friendly,
		&enemy,
		false,
		context.friendly_formation_id,
		context.engagement,
	);
	if let Some(round) = resolve_shelling_round(&mut friendly, &mut enemy, shelling_round_1) {
		hougeki1 = Some(round);
		hourai_flag[0] = 1;
	}

	if any_alive(&friendly) && any_alive(&enemy) {
		let shelling_round_2 = simulate_shelling_round(
			&enemy,
			&friendly,
			true,
			context.enemy_formation_id,
			context.engagement,
		);
		if let Some(round) = resolve_shelling_round(&mut friendly, &mut enemy, shelling_round_2) {
			hougeki2 = Some(round);
			hourai_flag[1] = 1;
		}
	}

	if any_alive(&friendly)
		&& any_alive(&enemy)
		&& (can_torpedo(&friendly) || can_torpedo(&enemy))
		&& let Some(round) = simulate_raigeki(
			&mut friendly,
			&mut enemy,
			context.friendly_formation_id,
			context.enemy_formation_id,
			context.engagement,
		) {
		raigeki = Some(round);
		hourai_flag[3] = 1;
	}

	let win_rank = calculate_win_rank(&friendly, &enemy);
	let can_midnight = any_alive(&friendly) && any_alive(&enemy);
	let packet = BattlePacket {
		formation: [
			context.friendly_formation_id,
			context.enemy_formation_id,
			context.engagement.api_id(),
		],
		friendly_nowhps: friendly.iter().map(|ship| ship.current_hp.max(0)).collect(),
		friendly_maxhps: friendly.iter().map(|ship| ship.ship.api_maxhp).collect(),
		enemy_nowhps: enemy.iter().map(|ship| ship.current_hp.max(0)).collect(),
		enemy_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
		smoke_type: 0,
		balloon_cell: 0,
		atoll_cell: 0,
		midnight_flag: 0,
		search: [1, 1],
		stage_flag,
		kouku,
		opening_taisen_flag: 0,
		opening_taisen: None,
		opening_flag: i64::from(opening_attack.is_some()),
		opening_attack,
		hourai_flag,
		hougeki1,
		hougeki2,
		hougeki3,
		raigeki,
	};

	let outcome = BattleOutcome {
		win_rank,
		mvp: calculate_mvp(&friendly),
		can_midnight,
	};

	let _ = context.mode;
	BattleSimulation {
		friendly,
		enemy,
		packet,
		outcome,
	}
}

pub fn apply_cap(raw_power: f64, cap: f64) -> i64 {
	if raw_power <= cap {
		raw_power.floor() as i64
	} else {
		(cap + (raw_power - cap).sqrt().floor()).floor() as i64
	}
}

fn simulate_shelling_round(
	attackers: &[BattleRuntimeShip],
	defenders: &[BattleRuntimeShip],
	attacker_enemy: bool,
	formation_id: i64,
	engagement: EngagementType,
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
			damage: calculate_shelling_damage(
				ship,
				&defenders[first_alive_defender],
				formation_id,
				engagement,
			),
		})
		.collect()
}

fn resolve_shelling_round(
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	round: Vec<PendingShell>,
) -> Option<BattleHougeki> {
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
		let target = if action.attacker_enemy {
			friendly.get_mut(action.defender_idx as usize)?
		} else {
			enemy.get_mut(action.defender_idx as usize)?
		};
		let dealt = action.damage.min(target.current_hp.max(0));
		target.current_hp -= dealt;

		at_eflag.push(i64::from(action.attacker_enemy));
		at_list.push(action.attacker_idx);
		at_type.push(0);
		df_list.push(vec![action.defender_idx]);
		si_list.push(action.weapon_ids);
		cl_list.push(vec![1]);
		damage.push(vec![dealt]);

		if !action.attacker_enemy
			&& let Some(attacker) = friendly.get_mut(action.attacker_idx as usize)
		{
			attacker.damage_dealt += dealt;
		}
	}

	Some(BattleHougeki {
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
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	friendly_formation_id: i64,
	enemy_formation_id: i64,
	engagement: EngagementType,
) -> Option<BattleOpeningAttack> {
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
			let dealt = calculate_torpedo_damage(
				ship,
				&enemy[target_idx],
				friendly_formation_id,
				engagement,
			);
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
			let dealt = calculate_torpedo_damage(
				ship,
				&friendly[target_idx],
				enemy_formation_id,
				engagement,
			);
			let dealt = dealt.min(friendly[target_idx].current_hp.max(0));
			friendly[target_idx].current_hp -= dealt;
			api_erai_list_items[idx] = Some(vec![target_idx as i64]);
			api_ecl_list_items[idx] = Some(vec![1]);
			api_fydam_list_items[idx] = Some(vec![dealt]);
			api_fdam[target_idx] += dealt;
			happened = true;
		}
	}

	happened.then_some(BattleOpeningAttack {
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
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	friendly_formation_id: i64,
	enemy_formation_id: i64,
	engagement: EngagementType,
) -> Option<BattleRaigeki> {
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
		let dealt =
			calculate_torpedo_damage(ship, &enemy[target_idx], friendly_formation_id, engagement)
				.min(enemy[target_idx].current_hp.max(0));
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
			calculate_torpedo_damage(ship, &friendly[target_idx], enemy_formation_id, engagement)
				.min(friendly[target_idx].current_hp.max(0));
		friendly[target_idx].current_hp -= dealt;
		api_erai[idx] = target_idx as i64;
		api_ecl[idx] = 1;
		api_fydam[idx] = dealt;
		api_fdam[target_idx] += dealt;
		happened = true;
	}

	happened.then_some(BattleRaigeki {
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
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
) -> BattleKouku {
	let friend_planes = total_plane_count(codex, friendly);
	let enemy_planes = total_plane_count(codex, enemy);
	let mut api_edam = vec![0; enemy.len()];
	let mut api_fdam = vec![0; friendly.len()];

	if friend_planes > 0
		&& let Some(target_idx) = enemy.iter().position(|ship| ship.current_hp > 0)
	{
		let dealt = 6.min(enemy[target_idx].current_hp.max(0));
		enemy[target_idx].current_hp -= dealt;
		api_edam[target_idx] = dealt;
	}
	if enemy_planes > 0
		&& let Some(target_idx) = friendly.iter().position(|ship| ship.current_hp > 0)
	{
		let dealt = 3.min(friendly[target_idx].current_hp.max(0));
		friendly[target_idx].current_hp -= dealt;
		api_fdam[target_idx] = dealt;
	}

	BattleKouku {
		api_plane_from: [plane_from(codex, friendly), plane_from(codex, enemy)],
		api_stage1: BattleKoukuStage1 {
			api_f_count: friend_planes,
			api_f_lostcount: 0,
			api_e_count: enemy_planes,
			api_e_lostcount: 0,
			api_disp_seiku: i64::from(friend_planes >= enemy_planes),
			api_touch_plane: [
				first_touch_plane(codex, friendly).unwrap_or(-1),
				first_touch_plane(codex, enemy).unwrap_or(-1),
			],
		},
		api_stage2: BattleKoukuStage2 {
			api_f_count: friend_planes,
			api_f_lostcount: friend_planes.min(4),
			api_e_count: enemy_planes,
			api_e_lostcount: enemy_planes.min(enemy_planes),
		},
		api_stage3: BattleKoukuStage3 {
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

fn calculate_shelling_damage(
	attacker: &BattleRuntimeShip,
	defender: &BattleRuntimeShip,
	formation_id: i64,
	engagement: EngagementType,
) -> i64 {
	let attack_power = (attacker.ship.api_karyoku[0].max(0) as f64 + 5.0)
		* shelling_formation_modifier(formation_id);
	let capped_power = apply_cap(attack_power * engagement.modifier(), 220.0) as f64;
	let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
	(capped_power - armor).floor().max(1.0) as i64
}

fn calculate_torpedo_damage(
	attacker: &BattleRuntimeShip,
	defender: &BattleRuntimeShip,
	formation_id: i64,
	engagement: EngagementType,
) -> i64 {
	let attack_power = (attacker.ship.api_raisou[0].max(0) as f64 + 5.0)
		* torpedo_formation_modifier(formation_id);
	let capped_power = apply_cap(attack_power * engagement.modifier(), 180.0) as f64;
	let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.55;
	(capped_power - armor).floor().max(1.0) as i64
}

fn shelling_formation_modifier(formation_id: i64) -> f64 {
	match formation_id {
		2 => 0.8,
		3 => 0.7,
		4 => 0.85,
		5 => 0.6,
		_ => 1.0,
	}
}

fn torpedo_formation_modifier(formation_id: i64) -> f64 {
	match formation_id {
		2 => 0.8,
		3 => 0.7,
		4 => 0.85,
		5 => 0.6,
		_ => 1.0,
	}
}

fn total_plane_count(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
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

fn has_any_aircraft(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
	total_plane_count(codex, ships) > 0
}

fn plane_from(codex: &Codex, ships: &[BattleRuntimeShip]) -> Vec<i64> {
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

fn first_touch_plane(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<i64> {
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

pub fn weapon_display_ids(ship: &BattleRuntimeShip) -> Vec<i64> {
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

fn can_opening_torpedo(ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0 && ship.ship.api_raisou[0] > 0)
}

fn can_torpedo(ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0 && ship.ship.api_raisou[0] > 0)
}

pub fn any_alive(ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0)
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

pub fn calculate_mvp(friendly: &[BattleRuntimeShip]) -> i64 {
	friendly
		.iter()
		.enumerate()
		.max_by_key(|(_, ship)| ship.damage_dealt)
		.map(|(idx, _)| idx as i64 + 1)
		.unwrap_or(-1)
}

pub fn calculate_win_rank(friendly: &[BattleRuntimeShip], enemy: &[BattleRuntimeShip]) -> String {
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
	use emukc_model::{codex::Codex, kc2::level};

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
	fn day_shelling_cap_matches_reference_example() {
		assert_eq!(apply_cap(250.0, 220.0), 225);
		assert_eq!(apply_cap(224.0, 220.0), 222);
	}

	#[test]
	fn battle_context_applies_formation_and_engagement() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, 89, 99));
		let mut defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99));
		attacker.ship.api_karyoku[0] = 180;
		defender.ship.api_soukou[0] = 20;
		let normal_damage =
			calculate_shelling_damage(&attacker, &defender, 1, EngagementType::SameCourse);
		let penalized_damage =
			calculate_shelling_damage(&attacker, &defender, 5, EngagementType::TDisadvantage);

		assert!(normal_damage > penalized_damage);
	}
}
