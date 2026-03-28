use serde::Serialize;

use emukc_model::{
	codex::Codex,
	kc2::{
		KcApiShip, KcApiSlotItem, KcShipType, KcSlotItemType3, KcSortieResultRank,
		start2::ApiMstSlotitem,
	},
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

	pub const fn from_api_id(api_id: i64) -> Option<Self> {
		match api_id {
			1 => Some(Self::SameCourse),
			2 => Some(Self::HeadOn),
			3 => Some(Self::TAdvantage),
			4 => Some(Self::TDisadvantage),
			_ => None,
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
pub struct BattleNightHougeki {
	pub api_at_eflag: Vec<i64>,
	pub api_at_list: Vec<i64>,
	pub api_n_mother_list: Vec<i64>,
	pub api_df_list: Vec<Vec<i64>>,
	pub api_si_list: Vec<Vec<i64>>,
	pub api_cl_list: Vec<Vec<i64>>,
	pub api_sp_list: Vec<i64>,
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
pub struct NightBattlePacket {
	pub formation: [i64; 3],
	pub friendly_nowhps: Vec<i64>,
	pub friendly_maxhps: Vec<i64>,
	pub enemy_nowhps: Vec<i64>,
	pub enemy_maxhps: Vec<i64>,
	pub touch_plane: [i64; 2],
	pub flare_pos: [i64; 2],
	pub hougeki: Option<BattleNightHougeki>,
}

#[derive(Debug, Clone)]
pub struct NightBattleSimulation {
	pub friendly: Vec<BattleRuntimeShip>,
	pub enemy: Vec<BattleRuntimeShip>,
	pub packet: NightBattlePacket,
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

	if has_any_air_combat_planes(codex, &friendly) || has_any_air_combat_planes(codex, &enemy) {
		stage_flag = [1, 1, 1];
		kouku = Some(simulate_kouku(codex, &mut friendly, &mut enemy));
	}

	if can_opening_torpedo(codex, &friendly) || can_opening_torpedo(codex, &enemy) {
		opening_attack = simulate_opening_torpedo(
			codex,
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
		codex,
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
			codex,
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
		&& (can_closing_torpedo(codex, &friendly) || can_closing_torpedo(codex, &enemy))
		&& let Some(round) = simulate_raigeki(
			codex,
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
		midnight_flag: i64::from(can_midnight),
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

pub fn simulate_night_battle_v1(
	codex: &Codex,
	mut friendly: Vec<BattleRuntimeShip>,
	mut enemy: Vec<BattleRuntimeShip>,
	friendly_formation_id: i64,
	enemy_formation_id: i64,
	engagement: EngagementType,
) -> NightBattleSimulation {
	let entry_friendly_nowhps =
		friendly.iter().map(|ship| ship.current_hp.max(0)).collect::<Vec<_>>();
	let entry_friendly_maxhps = friendly.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
	let entry_enemy_nowhps = enemy.iter().map(|ship| ship.current_hp.max(0)).collect::<Vec<_>>();
	let entry_enemy_maxhps = enemy.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
	let hougeki = simulate_night_hougeki(
		codex,
		&mut friendly,
		&mut enemy,
		friendly_formation_id,
		enemy_formation_id,
		engagement,
	);
	let outcome = BattleOutcome {
		win_rank: calculate_win_rank(&friendly, &enemy),
		mvp: calculate_mvp(&friendly),
		can_midnight: false,
	};
	let packet = NightBattlePacket {
		formation: [friendly_formation_id, enemy_formation_id, engagement.api_id()],
		friendly_nowhps: entry_friendly_nowhps,
		friendly_maxhps: entry_friendly_maxhps,
		enemy_nowhps: entry_enemy_nowhps,
		enemy_maxhps: entry_enemy_maxhps,
		touch_plane: [-1, -1],
		flare_pos: [-1, -1],
		hougeki,
	};

	NightBattleSimulation {
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
	codex: &Codex,
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
		.filter(|(_, ship)| can_shell_day_ship(codex, ship))
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
	codex: &Codex,
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
			if !can_opening_torpedo_ship(codex, ship) {
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
			if !can_opening_torpedo_ship(codex, ship) {
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
	codex: &Codex,
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
		if !can_closing_torpedo_ship(codex, ship) {
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
		if !can_closing_torpedo_ship(codex, ship) {
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
	let friend_lostcount = friend_planes.min(4);
	let enemy_lostcount = enemy_planes.min(enemy_planes);
	let mut api_edam = vec![0; enemy.len()];
	let mut api_fdam = vec![0; friendly.len()];

	if total_attack_plane_count(codex, friendly) > 0
		&& let Some(target_idx) = enemy.iter().position(|ship| ship.current_hp > 0)
	{
		let dealt = 6.min(enemy[target_idx].current_hp.max(0));
		enemy[target_idx].current_hp -= dealt;
		api_edam[target_idx] = dealt;
	}
	if total_attack_plane_count(codex, enemy) > 0
		&& let Some(target_idx) = friendly.iter().position(|ship| ship.current_hp > 0)
	{
		let dealt = 3.min(friendly[target_idx].current_hp.max(0));
		friendly[target_idx].current_hp -= dealt;
		api_fdam[target_idx] = dealt;
	}

	apply_plane_losses(codex, friendly, friend_lostcount);
	apply_plane_losses(codex, enemy, enemy_lostcount);

	BattleKouku {
		api_plane_from: [attack_plane_from(codex, friendly), attack_plane_from(codex, enemy)],
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
			api_f_lostcount: friend_lostcount,
			api_e_count: enemy_planes,
			api_e_lostcount: enemy_lostcount,
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

fn calculate_night_damage(
	attacker: &BattleRuntimeShip,
	defender: &BattleRuntimeShip,
	engagement: EngagementType,
) -> i64 {
	let attack_power =
		(attacker.ship.api_karyoku[0].max(0) + attacker.ship.api_raisou[0].max(0) + 5) as f64;
	let capped_power = apply_cap(attack_power * engagement.modifier(), 360.0) as f64;
	let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
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
					.is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
		})
		.map(|(_, onslot)| onslot)
		.sum()
}

fn total_attack_plane_count(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
	ships
		.iter()
		.flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
		.filter(|(slot_item, onslot)| {
			*onslot > 0
				&& codex
					.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
					.ok()
					.is_some_and(|mst| is_airstrike_attack_type(mst.api_type[2]))
		})
		.map(|(_, onslot)| onslot)
		.sum()
}

fn has_any_air_combat_planes(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
	total_plane_count(codex, ships) > 0
}

fn attack_plane_from(codex: &Codex, ships: &[BattleRuntimeShip]) -> Vec<i64> {
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
							.is_some_and(|mst| is_airstrike_attack_type(mst.api_type[2]))
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

fn can_opening_torpedo(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| can_opening_torpedo_ship(codex, ship))
}

fn can_closing_torpedo(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| can_closing_torpedo_ship(codex, ship))
}

pub fn any_alive(ships: &[BattleRuntimeShip]) -> bool {
	ships.iter().any(|ship| ship.current_hp > 0)
}

fn is_air_combat_type(slotitem_type: i64) -> bool {
	matches!(
		KcSlotItemType3::n(slotitem_type),
		Some(
			KcSlotItemType3::CarrierBasedFighter
				| KcSlotItemType3::CarrierBasedDiveBomber
				| KcSlotItemType3::CarrierBasedTorpedoBomber
				| KcSlotItemType3::CarrierBasedRecon
				| KcSlotItemType3::CarrierBasedRecon2
				| KcSlotItemType3::SeaBasedBomber
				| KcSlotItemType3::SeaBasedRecon
				| KcSlotItemType3::SeaplaneFighter
				| KcSlotItemType3::JetFighter
				| KcSlotItemType3::JetFighterBomber
				| KcSlotItemType3::JetAttacker
				| KcSlotItemType3::JetRecon
		)
	)
}

fn is_airstrike_attack_type(slotitem_type: i64) -> bool {
	matches!(
		KcSlotItemType3::n(slotitem_type),
		Some(
			KcSlotItemType3::CarrierBasedDiveBomber
				| KcSlotItemType3::CarrierBasedTorpedoBomber
				| KcSlotItemType3::SeaBasedBomber
				| KcSlotItemType3::JetFighterBomber
				| KcSlotItemType3::JetAttacker
		)
	)
}

fn ship_type(codex: &Codex, ship: &BattleRuntimeShip) -> Option<KcShipType> {
	KcShipType::n(codex.get_ship_type(ship.ship.api_ship_id) as i32)
}

fn has_slotitem_type(codex: &Codex, ship: &BattleRuntimeShip, wanted: KcSlotItemType3) -> bool {
	ship.slot_items.iter().any(|slot_item| {
		codex
			.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
			.ok()
			.and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
			== Some(wanted)
	})
}

fn has_slotitem_id(ship: &BattleRuntimeShip, wanted: i64) -> bool {
	ship.slot_items.iter().any(|slot_item| slot_item.api_slotitem_id == wanted)
}

fn can_opening_torpedo_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(KcShipType::CLT | KcShipType::SS | KcShipType::SSV) => true,
		_ => has_slotitem_type(codex, ship, KcSlotItemType3::SpecialSubmarineVessel),
	}
}

fn can_closing_torpedo_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.current_hp <= 0 || ship.ship.api_raisou[0] <= 0 {
		return false;
	}

	matches!(
		ship_type(codex, ship),
		Some(
			KcShipType::DE
				| KcShipType::DD
				| KcShipType::CL
				| KcShipType::CLT
				| KcShipType::CA
				| KcShipType::CAV
				| KcShipType::AV
				| KcShipType::LHA
				| KcShipType::SS
				| KcShipType::SSV
				| KcShipType::CT
				| KcShipType::AO
		)
	)
}

fn can_shell_day_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.current_hp <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(KcShipType::SS | KcShipType::SSV | KcShipType::AS) => false,
		Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
			total_attack_plane_count(codex, std::slice::from_ref(ship)) > 0
		}
		_ => true,
	}
}

fn can_attack_night_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.current_hp <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
			(has_slotitem_id(ship, 258) || has_slotitem_id(ship, 259))
				&& ship.slot_items.iter().any(|slot_item| {
					codex
						.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
						.ok()
						.is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
				})
		}
		Some(KcShipType::SS | KcShipType::SSV | KcShipType::AS) => false,
		_ => true,
	}
}

fn apply_plane_losses(codex: &Codex, ships: &mut [BattleRuntimeShip], mut lostcount: i64) {
	while lostcount > 0 {
		let mut best_slot: Option<(usize, usize, i64)> = None;
		for (ship_idx, ship) in ships.iter().enumerate() {
			for (slot_idx, slot_item) in ship.slot_items.iter().enumerate().take(5) {
				let onslot = ship.ship.api_onslot[slot_idx];
				if onslot <= 0 {
					continue;
				}
				let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()
				else {
					continue;
				};
				if !is_air_combat_type(mst.api_type[2]) {
					continue;
				}
				if best_slot.is_none_or(|(_, _, current)| onslot > current) {
					best_slot = Some((ship_idx, slot_idx, onslot));
				}
			}
		}

		let Some((ship_idx, slot_idx, _)) = best_slot else {
			break;
		};
		ships[ship_idx].ship.api_onslot[slot_idx] -= 1;
		lostcount -= 1;
	}
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

fn simulate_night_hougeki(
	codex: &Codex,
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	friendly_formation_id: i64,
	enemy_formation_id: i64,
	engagement: EngagementType,
) -> Option<BattleNightHougeki> {
	let mut at_eflag = Vec::new();
	let mut at_list = Vec::new();
	let mut n_mother_list = Vec::new();
	let mut df_list = Vec::new();
	let mut si_list = Vec::new();
	let mut cl_list = Vec::new();
	let mut sp_list = Vec::new();
	let mut damage = Vec::new();

	for (idx, ship) in friendly.iter_mut().enumerate() {
		if !can_attack_night_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) = enemy.iter().position(|target| target.current_hp > 0) else {
			break;
		};
		let dealt = calculate_night_damage(ship, &enemy[target_idx], engagement)
			.min(enemy[target_idx].current_hp.max(0));
		enemy[target_idx].current_hp -= dealt;
		ship.damage_dealt += dealt;
		at_eflag.push(0);
		at_list.push(idx as i64);
		n_mother_list.push(0);
		df_list.push(vec![target_idx as i64]);
		si_list.push(weapon_display_ids(ship));
		cl_list.push(vec![1]);
		sp_list.push(0);
		damage.push(vec![dealt]);
	}

	for (idx, ship) in enemy.iter_mut().enumerate() {
		if !can_attack_night_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) = friendly.iter().position(|target| target.current_hp > 0) else {
			break;
		};
		let dealt = calculate_night_damage(ship, &friendly[target_idx], engagement)
			.min(friendly[target_idx].current_hp.max(0));
		friendly[target_idx].current_hp -= dealt;
		at_eflag.push(1);
		at_list.push(idx as i64);
		n_mother_list.push(0);
		df_list.push(vec![target_idx as i64]);
		si_list.push(weapon_display_ids(ship));
		cl_list.push(vec![1]);
		sp_list.push(0);
		damage.push(vec![dealt]);
	}

	if at_list.is_empty() {
		return None;
	}

	let _ = (friendly_formation_id, enemy_formation_id);
	Some(BattleNightHougeki {
		api_at_eflag: at_eflag,
		api_at_list: at_list,
		api_n_mother_list: n_mother_list,
		api_df_list: df_list,
		api_si_list: si_list,
		api_cl_list: cl_list,
		api_sp_list: sp_list,
		api_damage: damage,
	})
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

	fn first_ship_mst_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
		codex
			.manifest
			.api_mst_ship
			.iter()
			.find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
			.map(|mst| mst.api_id)
			.unwrap()
	}

	fn first_slotitem_mst_by_type(codex: &Codex, slot_type: KcSlotItemType3) -> i64 {
		codex
			.manifest
			.api_mst_slotitem
			.iter()
			.find(|mst| KcSlotItemType3::n(mst.api_type[2]) == Some(slot_type))
			.map(|mst| mst.api_id)
			.unwrap()
	}

	fn slotitem_with_mst_id(mst_id: i64) -> KcApiSlotItem {
		KcApiSlotItem {
			api_id: 0,
			api_slotitem_id: mst_id,
			api_locked: 0,
			api_level: 0,
			api_alv: None,
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

	#[test]
	fn sortie_day_battle_enables_midnight_when_both_sides_survive() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let mut friend = sample_ship(&codex, 79, 1);
		friend.ship.api_karyoku[0] = 1;
		friend.ship.api_raisou[0] = 0;
		friend.ship.api_soukou[0] = 200;

		let mut enemy = sample_ship(&codex, 412, 99);
		enemy.ship.api_karyoku[0] = 1;
		enemy.ship.api_raisou[0] = 0;
		enemy.ship.api_soukou[0] = 200;

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Sortie,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![friend],
				enemy_ships: vec![enemy],
			},
		);

		assert_eq!(simulation.packet.midnight_flag, 1);
		assert!(simulation.outcome.can_midnight);
	}

	#[test]
	fn fighter_only_carrier_does_not_launch_airstrike_damage() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);

		let mut carrier = sample_ship(&codex, carrier_mst, 50);
		carrier.slot_items = vec![slotitem_with_mst_id(fighter_id)];
		carrier.ship.api_onslot = [18, 0, 0, 0, 0];
		let enemy = sample_ship(&codex, dd_mst, 50);

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Practice,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![carrier],
				enemy_ships: vec![enemy],
			},
		);

		let kouku = simulation.packet.kouku.unwrap();
		assert!(kouku.api_plane_from[0].is_empty());
		assert_eq!(kouku.api_stage3.api_edam.iter().sum::<i64>(), 0);
	}

	#[test]
	fn only_opening_torpedo_capable_ship_participates() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

		let dd = sample_ship(&codex, dd_mst, 50);
		let clt = sample_ship(&codex, clt_mst, 50);
		let enemy = sample_ship(&codex, bb_mst, 50);

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Practice,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![dd, clt],
				enemy_ships: vec![enemy],
			},
		);

		let opening = simulation.packet.opening_attack.unwrap();
		assert!(opening.api_frai_list_items[0].is_none());
		assert!(opening.api_frai_list_items[1].is_some());
	}

	#[test]
	fn fighter_only_carrier_does_not_shell_in_day_battle() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);

		let mut carrier = sample_ship(&codex, carrier_mst, 50);
		carrier.slot_items = vec![slotitem_with_mst_id(fighter_id)];
		carrier.ship.api_onslot = [18, 0, 0, 0, 0];
		let bb = sample_ship(&codex, bb_mst, 50);
		let enemy = sample_ship(&codex, dd_mst, 50);

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Practice,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![carrier, bb],
				enemy_ships: vec![enemy],
			},
		);

		let hougeki = simulation.packet.hougeki1.unwrap();
		assert_eq!(hougeki.api_at_list, vec![1]);
	}

	#[test]
	fn regular_carrier_cannot_attack_in_night_battle_without_night_crew() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

		let carrier = sample_ship(&codex, carrier_mst, 50);
		let enemy = sample_ship(&codex, dd_mst, 50);

		let simulation = simulate_night_battle_v1(
			&codex,
			vec![BattleRuntimeShip::from(carrier)],
			vec![BattleRuntimeShip::from(enemy)],
			1,
			1,
			EngagementType::SameCourse,
		);

		let hougeki = simulation.packet.hougeki.unwrap();
		assert!(hougeki.api_at_eflag.iter().all(|flag| *flag == 1));
	}
}
