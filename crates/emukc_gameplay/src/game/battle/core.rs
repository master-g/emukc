use rand::{RngExt, SeedableRng, rng, rngs::StdRng};
use serde::Serialize;

use emukc_model::{
	codex::Codex,
	kc2::{
		KcApiShip, KcApiSlotItem, KcShipType, KcSlotItemType3, KcSortieResultRank,
		start2::ApiMstSlotitem,
	},
};

const DAY_SURFACE_DISPLAY_TYPES: &[KcSlotItemType3] = &[
	KcSlotItemType3::SmallCaliberMainGun,
	KcSlotItemType3::MediumCaliberMainGun,
	KcSlotItemType3::LargeCaliberMainGun,
	KcSlotItemType3::SecondaryGun,
	KcSlotItemType3::LargeCaliberMainGun2,
	KcSlotItemType3::SecondaryGun2,
	KcSlotItemType3::Torpedo,
	KcSlotItemType3::SubmarineTorpedo,
	KcSlotItemType3::CarrierBasedDiveBomber,
	KcSlotItemType3::CarrierBasedTorpedoBomber,
	KcSlotItemType3::SeaBasedBomber,
	KcSlotItemType3::JetFighterBomber,
	KcSlotItemType3::JetAttacker,
];

const ASW_DISPLAY_TYPES: &[KcSlotItemType3] = &[
	KcSlotItemType3::Sonar,
	KcSlotItemType3::LargeSonar,
	KcSlotItemType3::DepthCharge,
	KcSlotItemType3::AutoGyro,
	KcSlotItemType3::AntiSubmarinePatrol,
	KcSlotItemType3::SeaBasedBomber,
	KcSlotItemType3::LargeFlyingBoat,
];

const NIGHT_MAIN_GUN_TYPES: &[KcSlotItemType3] = &[
	KcSlotItemType3::SmallCaliberMainGun,
	KcSlotItemType3::MediumCaliberMainGun,
	KcSlotItemType3::LargeCaliberMainGun,
	KcSlotItemType3::LargeCaliberMainGun2,
];

const NIGHT_SECONDARY_GUN_TYPES: &[KcSlotItemType3] =
	&[KcSlotItemType3::SecondaryGun, KcSlotItemType3::SecondaryGun2];

const NIGHT_TORPEDO_TYPES: &[KcSlotItemType3] =
	&[KcSlotItemType3::Torpedo, KcSlotItemType3::SubmarineTorpedo];

const RADAR_DISPLAY_TYPES: &[KcSlotItemType3] =
	&[KcSlotItemType3::SmallRadar, KcSlotItemType3::LargeRadar, KcSlotItemType3::LargeRadar2];

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleMode {
	Practice,
	Sortie,
}

/// Controls which phases execute in a day battle simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleType {
	/// Normal day battle: kouku → OASW → opening torpedo → shelling × 2 → closing torpedo.
	Normal,
	/// Air battle only (航空戦): kouku + OASW, no shelling / torpedo.
	AirBattle,
	/// Long-distance air raid (長距離空襲): kouku only, no OASW / shelling / torpedo.
	LdAirBattle,
	/// Long-distance shelling (長距離砲撃): shelling only, no kouku / torpedo.
	LdShooting,
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
	/// Current HP — only mutable through [`apply_damage`](Self::apply_damage).
	current_hp: i64,
	/// HP at the start of this battle node (before any combat phases).
	/// Used to determine sinking protection eligibility.
	pub entry_hp: i64,
	pub damage_dealt: i64,
	/// Whether this ship belongs to the player (friendly) side.
	is_friendly: bool,
	/// Whether this battle is a sortie (true) or practice (false).
	/// Sinking protection only applies during sorties.
	is_sortie: bool,
}

impl BattleRuntimeShip {
	/// Create a runtime ship for battle simulation.
	pub fn new(input: BattleShipInput, is_friendly: bool, is_sortie: bool) -> Self {
		Self {
			current_hp: input.ship.api_nowhp,
			entry_hp: input.ship.api_nowhp,
			damage_dealt: 0,
			ship: input.ship,
			slot_items: input.slot_items,
			effect_list: input.effect_list,
			is_friendly,
			is_sortie,
		}
	}

	/// Current HP (read-only).
	pub fn hp(&self) -> i64 {
		self.current_hp
	}

	pub fn is_alive(&self) -> bool {
		self.current_hp > 0
	}

	pub fn is_sunk(&self) -> bool {
		self.current_hp <= 0
	}

	/// Apply damage with sinking protection (轟沈ストッパー).
	///
	/// In real KanColle:
	/// - Friendly ships that were **not** in taiha (HP ≤ 25% max) at the start of
	///   the battle node cannot be sunk. Lethal damage is replaced with
	///   proportional damage: `floor(0.5 * H + 0.3 * rand(0..H))`.
	/// - The flagship (index 0) can **never** be sunk regardless of HP state.
	/// - Protection only applies to friendly ships during sorties (not practice).
	///
	/// Returns the actual damage dealt (after any protection replacement).
	pub fn apply_damage(
		&mut self,
		random: &mut BattleRandom,
		raw_damage: i64,
		ship_index: usize,
	) -> i64 {
		if self.is_sunk() {
			return 0;
		}

		let effective = raw_damage.min(self.current_hp);

		// Sinking protection only applies to friendly ships during sorties.
		if self.is_friendly && self.is_sortie && effective >= self.current_hp {
			let is_flagship = ship_index == 0;
			// Taiha threshold: HP ≤ 25% of max at node entry.
			let was_taiha_at_entry = self.entry_hp * 4 <= self.ship.api_maxhp;
			let is_protected = is_flagship || !was_taiha_at_entry;

			if is_protected {
				// Replace lethal damage with proportional damage (割合ダメージ).
				// Formula: floor(0.5 * H + 0.3 * rand(0..H)), clamped to [0, H-1].
				let h = self.current_hp;
				let rand_part = if h > 1 { random.roll_range(0, h) } else { 0 };
				let proportional = (0.5 * h as f64 + 0.3 * rand_part as f64).floor() as i64;
				let dealt = proportional.clamp(0, h - 1);
				self.current_hp -= dealt;
				return dealt;
			}
		}

		self.current_hp -= effective;
		effective
	}
}

impl From<BattleShipInput> for BattleRuntimeShip {
	/// Convenience conversion for tests and non-protection contexts.
	/// Defaults to enemy ship in non-sortie (no sinking protection).
	fn from(input: BattleShipInput) -> Self {
		Self::new(input, false, false)
	}
}

#[derive(Debug, Clone)]
pub struct BattleContext {
	pub mode: BattleMode,
	pub battle_type: BattleType,
	/// Whether this is a sortie battle (true) or practice (false).
	/// Sinking protection only applies during sorties.
	pub is_sortie: bool,
	pub friendly_formation_id: i64,
	pub enemy_formation_id: i64,
	pub engagement: EngagementType,
	pub friend_ships: Vec<BattleShipInput>,
	pub enemy_ships: Vec<BattleShipInput>,
	pub rng_seed: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattlePhase {
	OpeningTorpedo,
	DayShelling,
	ClosingTorpedo,
	NightShelling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetClass {
	Surface,
	Submarine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AirState {
	Supremacy,
	Superiority,
	Parity,
	Denial,
	Incapability,
}

impl AirState {
	fn from_power(friendly: i64, enemy: i64) -> Self {
		if enemy == 0 && friendly == 0 {
			return Self::Parity;
		}
		if enemy == 0 {
			return Self::Supremacy;
		}
		// Thresholds ordered from most favorable to least:
		// Supremacy:    friendly ≥ 3 × enemy
		// Superiority:  friendly ≥ 1.5 × enemy  (2*friendly ≥ 3*enemy)
		// ... Parity in the middle ...
		// Denial:       enemy ≥ 1.5 × friendly  (3*friendly ≤ 2*enemy)
		// Incapability: enemy ≥ 3 × friendly    (3*friendly ≤ enemy)
		if friendly >= 3 * enemy {
			Self::Supremacy
		} else if 2 * friendly >= 3 * enemy {
			Self::Superiority
		} else if 3 * friendly <= enemy {
			Self::Incapability
		} else if 3 * friendly <= 2 * enemy {
			Self::Denial
		} else {
			Self::Parity
		}
	}

	fn api_disp_seiku(self) -> i64 {
		match self {
			Self::Supremacy => 1,
			Self::Superiority => 2,
			Self::Parity => 0,
			Self::Denial => 3,
			Self::Incapability => 4,
		}
	}

	fn stage1_friendly_loss_ratio(self) -> (f64, f64) {
		match self {
			Self::Supremacy => (0.0, 0.04),
			Self::Superiority => (0.02, 0.08),
			Self::Parity => (0.04, 0.12),
			Self::Denial => (0.08, 0.18),
			Self::Incapability => (0.20, 0.36),
		}
	}

	fn stage1_enemy_loss_ratio(self) -> (f64, f64) {
		match self {
			Self::Supremacy => (0.20, 0.36),
			Self::Superiority => (0.08, 0.18),
			Self::Parity => (0.04, 0.12),
			Self::Denial => (0.02, 0.08),
			Self::Incapability => (0.0, 0.04),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttackCapability {
	CannotAttack,
	SurfaceOnly,
	BothPreferSubmarine,
}

#[derive(Debug)]
struct BattleRandom {
	seeded: Option<StdRng>,
}

impl BattleRandom {
	fn new(seed: Option<u64>) -> Self {
		Self {
			seeded: seed.map(StdRng::seed_from_u64),
		}
	}

	fn choose_index(&mut self, len: usize) -> usize {
		debug_assert!(len > 0);
		if let Some(seed) = &mut self.seeded {
			seed.random_range(0..len)
		} else {
			rng().random_range(0..len)
		}
	}

	fn roll_scratch_damage(&mut self, current_hp: i64) -> i64 {
		let current_hp = current_hp.max(1);
		let random_part = if current_hp <= 1 {
			0
		} else if let Some(seed) = &mut self.seeded {
			seed.random_range(0..current_hp)
		} else {
			rng().random_range(0..current_hp)
		};

		((current_hp as f64) * 0.06 + (random_part as f64) * 0.08).floor().max(1.0) as i64
	}

	fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
		debug_assert!(min <= max);
		let r: f64 = if let Some(seed) = &mut self.seeded {
			seed.random_range(0u32..10001) as f64 / 10000.0
		} else {
			rng().random_range(0u32..10001) as f64 / 10000.0
		};
		min + r * (max - min)
	}

	/// Return a random i64 in `[min, max)`.  Handles `min >= max` gracefully.
	fn roll_range(&mut self, min: i64, max: i64) -> i64 {
		if min >= max {
			return min;
		}
		if let Some(seed) = &mut self.seeded {
			seed.random_range(min..max)
		} else {
			rng().random_range(min..max)
		}
	}
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

pub fn simulate_day_battle_v1(codex: &Codex, context: BattleContext) -> BattleSimulation {
	let mut random = BattleRandom::new(context.rng_seed);
	let is_sortie = context.is_sortie;
	let mut friendly = context
		.friend_ships
		.into_iter()
		.map(|s| BattleRuntimeShip::new(s, true, is_sortie))
		.collect::<Vec<_>>();
	let mut enemy = context
		.enemy_ships
		.into_iter()
		.map(|s| BattleRuntimeShip::new(s, false, is_sortie))
		.collect::<Vec<_>>();

	let battle_type = context.battle_type;
	let run_kouku =
		matches!(battle_type, BattleType::Normal | BattleType::AirBattle | BattleType::LdAirBattle);
	let run_oasw = matches!(battle_type, BattleType::Normal | BattleType::AirBattle);
	let run_shelling = matches!(battle_type, BattleType::Normal | BattleType::LdShooting);
	let run_torpedo = matches!(battle_type, BattleType::Normal);

	let mut opening_attack = None;
	let mut hougeki1 = None;
	let mut hougeki2 = None;
	let hougeki3 = None;
	let mut raigeki = None;
	let mut kouku = None;
	let mut stage_flag = [0, 0, 0];
	let mut hourai_flag = [0, 0, 0, 0];
	let mut opening_taisen = None;
	let mut opening_taisen_flag = 0;

	if run_kouku
		&& (has_any_air_combat_planes(codex, &friendly) || has_any_air_combat_planes(codex, &enemy))
	{
		stage_flag = [1, 1, 1];
		kouku = Some(simulate_kouku(codex, &mut friendly, &mut enemy, &mut random));
	}

	if run_oasw {
		opening_taisen = simulate_opening_taisen(
			codex,
			&mut random,
			&mut friendly,
			&mut enemy,
			context.friendly_formation_id,
			context.enemy_formation_id,
			context.engagement,
		);
		opening_taisen_flag = i64::from(opening_taisen.is_some());
	}

	if run_torpedo && (can_opening_torpedo(codex, &friendly) || can_opening_torpedo(codex, &enemy))
	{
		opening_attack = simulate_opening_torpedo(
			codex,
			&mut random,
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

	if run_shelling {
		hougeki1 = simulate_shelling_side(
			codex,
			&mut random,
			&mut friendly,
			&mut enemy,
			false,
			context.friendly_formation_id,
			context.engagement,
			BattlePhase::DayShelling,
		);
		if hougeki1.is_some() {
			hourai_flag[0] = 1;
		}

		if any_alive(&friendly) && any_alive(&enemy) {
			hougeki2 = simulate_shelling_side(
				codex,
				&mut random,
				&mut enemy,
				&mut friendly,
				true,
				context.enemy_formation_id,
				context.engagement,
				BattlePhase::DayShelling,
			);
			if hougeki2.is_some() {
				hourai_flag[1] = 1;
			}
		}
	}

	if run_torpedo
		&& any_alive(&friendly)
		&& any_alive(&enemy)
		&& (can_closing_torpedo(codex, &friendly) || can_closing_torpedo(codex, &enemy))
		&& let Some(round) = simulate_raigeki(
			codex,
			&mut random,
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
	// LdAirBattle and LdShooting never lead to night battle.
	let can_midnight = matches!(battle_type, BattleType::Normal | BattleType::AirBattle)
		&& any_alive(&friendly)
		&& any_alive(&enemy);
	let packet = BattlePacket {
		formation: [
			context.friendly_formation_id,
			context.enemy_formation_id,
			context.engagement.api_id(),
		],
		friendly_nowhps: friendly.iter().map(|ship| ship.hp().max(0)).collect(),
		friendly_maxhps: friendly.iter().map(|ship| ship.ship.api_maxhp).collect(),
		enemy_nowhps: enemy.iter().map(|ship| ship.hp().max(0)).collect(),
		enemy_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
		smoke_type: 0,
		balloon_cell: 0,
		atoll_cell: 0,
		midnight_flag: i64::from(can_midnight),
		search: [1, 1],
		stage_flag,
		kouku,
		opening_taisen_flag,
		opening_taisen,
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

	verify_protected_ships_alive(&friendly);

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
	let mut random = BattleRandom::new(None);
	let entry_friendly_nowhps =
		friendly.iter().map(|ship| ship.hp().max(0)).collect::<Vec<_>>();
	let entry_friendly_maxhps = friendly.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
	let entry_enemy_nowhps = enemy.iter().map(|ship| ship.hp().max(0)).collect::<Vec<_>>();
	let entry_enemy_maxhps = enemy.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
	let hougeki = simulate_night_hougeki(
		codex,
		&mut random,
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

	verify_protected_ships_alive(&friendly);

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

fn simulate_shelling_side(
	codex: &Codex,
	random: &mut BattleRandom,
	attackers: &mut [BattleRuntimeShip],
	defenders: &mut [BattleRuntimeShip],
	attacker_enemy: bool,
	formation_id: i64,
	engagement: EngagementType,
	phase: BattlePhase,
) -> Option<BattleHougeki> {
	let mut at_eflag = Vec::new();
	let mut at_list = Vec::new();
	let mut at_type = Vec::new();
	let mut df_list = Vec::new();
	let mut si_list = Vec::new();
	let mut cl_list = Vec::new();
	let mut damage = Vec::new();

	for (idx, ship) in attackers.iter_mut().enumerate() {
		if !can_shell_day_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) = select_random_target_index(codex, random, ship, defenders, phase)
		else {
			continue;
		};
		let is_asw_attack = target_class(codex, &defenders[target_idx]) == TargetClass::Submarine;
		let raw = if is_asw_attack {
			calculate_asw_damage(codex, ship, &defenders[target_idx], formation_id, engagement)
		} else {
			calculate_shelling_damage(ship, &defenders[target_idx], formation_id, engagement)
		};
		let dealt = defenders[target_idx].apply_damage(random, raw, target_idx);
		if !attacker_enemy {
			ship.damage_dealt += dealt;
		}

		at_eflag.push(i64::from(attacker_enemy));
		at_list.push(idx as i64);
		at_type.push(if is_asw_attack {
			7
		} else {
			0
		});
		df_list.push(vec![target_idx as i64]);
		si_list.push(day_attack_display_ids(codex, ship, is_asw_attack));
		cl_list.push(vec![1]);
		damage.push(vec![dealt]);
	}

	(!at_list.is_empty()).then_some(BattleHougeki {
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
	random: &mut BattleRandom,
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

	for (idx, ship) in friendly.iter_mut().enumerate() {
		if !can_opening_torpedo_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, enemy, BattlePhase::OpeningTorpedo)
		else {
			continue;
		};
		let raw =
			calculate_torpedo_damage(ship, &enemy[target_idx], friendly_formation_id, engagement);
		let dealt =
			enemy[target_idx].apply_damage(random, raw, target_idx);
		ship.damage_dealt += dealt;
		api_frai_list_items[idx] = Some(vec![target_idx as i64]);
		api_fcl_list_items[idx] = Some(vec![1]);
		api_fydam_list_items[idx] = Some(vec![dealt]);
		api_edam[target_idx] += dealt;
		happened = true;
	}

	for (idx, ship) in enemy.iter_mut().enumerate() {
		if !can_opening_torpedo_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, friendly, BattlePhase::OpeningTorpedo)
		else {
			continue;
		};
		let raw =
			calculate_torpedo_damage(ship, &friendly[target_idx], enemy_formation_id, engagement);
		let dealt =
			friendly[target_idx].apply_damage(random, raw, target_idx);
		api_erai_list_items[idx] = Some(vec![target_idx as i64]);
		api_ecl_list_items[idx] = Some(vec![1]);
		api_eydam_list_items[idx] = Some(vec![dealt]);
		api_fdam[target_idx] += dealt;
		happened = true;
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
	random: &mut BattleRandom,
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
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, enemy, BattlePhase::ClosingTorpedo)
		else {
			continue;
		};
		let raw =
			calculate_torpedo_damage(ship, &enemy[target_idx], friendly_formation_id, engagement);
		let dealt =
			enemy[target_idx].apply_damage(random, raw, target_idx);
		ship.damage_dealt += dealt;
		api_frai[idx] = target_idx as i64;
		api_fcl[idx] = 1;
		api_fydam[idx] = dealt;
		api_edam[target_idx] += dealt;
		happened = true;
	}

	for (idx, ship) in enemy.iter_mut().enumerate() {
		if !can_closing_torpedo_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, friendly, BattlePhase::ClosingTorpedo)
		else {
			continue;
		};
		let raw =
			calculate_torpedo_damage(ship, &friendly[target_idx], enemy_formation_id, engagement);
		let dealt =
			friendly[target_idx].apply_damage(random, raw, target_idx);
		api_erai[idx] = target_idx as i64;
		api_ecl[idx] = 1;
		api_eydam[idx] = dealt;
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

fn is_fighter_power_type(slotitem_type: i64) -> bool {
	matches!(
		KcSlotItemType3::n(slotitem_type),
		Some(
			KcSlotItemType3::CarrierBasedFighter
				| KcSlotItemType3::CarrierBasedDiveBomber
				| KcSlotItemType3::CarrierBasedTorpedoBomber
				| KcSlotItemType3::SeaBasedBomber
				| KcSlotItemType3::SeaplaneFighter
				| KcSlotItemType3::JetFighter
				| KcSlotItemType3::JetFighterBomber
				| KcSlotItemType3::JetAttacker
		)
	)
}

fn calculate_fighter_power(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
	ships
		.iter()
		.flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
		.filter_map(|(slot_item, onslot)| {
			if onslot <= 0 {
				return None;
			}
			let mst = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()?;
			if !is_fighter_power_type(mst.api_type[2]) {
				return None;
			}
			let aa = mst.api_tyku.max(0) as f64;
			Some((aa * (onslot as f64).sqrt()).floor() as i64)
		})
		.sum()
}

fn calculate_airstrike_damage(
	codex: &Codex,
	attacker_ships: &[BattleRuntimeShip],
	defender: &BattleRuntimeShip,
) -> i64 {
	let total_bomb_power: f64 = attacker_ships
		.iter()
		.flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
		.filter_map(|(slot_item, onslot)| {
			if onslot <= 0 {
				return None;
			}
			let mst = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()?;
			if !is_airstrike_attack_type(mst.api_type[2]) {
				return None;
			}
			let is_torpedo_bomber = KcSlotItemType3::n(mst.api_type[2])
				== Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
			let stat = if is_torpedo_bomber {
				mst.api_raig.max(0) as f64
			} else {
				mst.api_baku.max(0) as f64
			};
			Some(stat * (onslot as f64).sqrt())
		})
		.sum();

	if total_bomb_power <= 0.0 {
		return 0;
	}
	let raw_power = total_bomb_power + 25.0;
	let capped = apply_cap(raw_power, 170.0) as f64;
	let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.6;
	(capped - armor).floor().max(1.0) as i64
}

fn simulate_kouku(
	codex: &Codex,
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	random: &mut BattleRandom,
) -> BattleKouku {
	let friend_planes = total_plane_count(codex, friendly);
	let enemy_planes = total_plane_count(codex, enemy);

	let friend_fighter_power = calculate_fighter_power(codex, friendly);
	let enemy_fighter_power = calculate_fighter_power(codex, enemy);
	let air_state = AirState::from_power(friend_fighter_power, enemy_fighter_power);

	// Stage 1: fighter combat — proportional losses based on air state
	let (f_loss_min, f_loss_max) = air_state.stage1_friendly_loss_ratio();
	let (e_loss_min, e_loss_max) = air_state.stage1_enemy_loss_ratio();
	let f_loss_ratio = random.random_f64_range(f_loss_min, f_loss_max);
	let e_loss_ratio = random.random_f64_range(e_loss_min, e_loss_max);
	let stage1_f_lost = (friend_planes as f64 * f_loss_ratio).floor() as i64;
	let stage1_e_lost = (enemy_planes as f64 * e_loss_ratio).floor() as i64;

	apply_plane_losses(codex, friendly, stage1_f_lost);
	apply_plane_losses(codex, enemy, stage1_e_lost);

	// Stage 2: anti-air fire — simplified proportional model.
	// NOTE: Real KanColle uses per-ship AA with slot-level shootdowns and fleet AA modifiers.
	// This linear approximation (total_aa / 400 × plane_count) is a known simplification.
	// Should be replaced with per-ship AA calculation before implementing airbattle / ld_airbattle.
	let friend_planes_after_s1 = total_plane_count(codex, friendly);
	let enemy_planes_after_s1 = total_plane_count(codex, enemy);
	let friendly_aa: f64 = friendly.iter().map(|s| s.ship.api_taiku[0].max(0) as f64).sum();
	let enemy_aa: f64 = enemy.iter().map(|s| s.ship.api_taiku[0].max(0) as f64).sum();
	let stage2_f_lost = ((enemy_aa / 400.0) * friend_planes_after_s1 as f64)
		.floor()
		.min(friend_planes_after_s1 as f64) as i64;
	let stage2_e_lost = ((friendly_aa / 400.0) * enemy_planes_after_s1 as f64)
		.floor()
		.min(enemy_planes_after_s1 as f64) as i64;

	apply_plane_losses(codex, friendly, stage2_f_lost);
	apply_plane_losses(codex, enemy, stage2_e_lost);

	// Stage 3: bombing damage
	let mut api_edam = vec![0i64; enemy.len()];
	let mut api_fdam = vec![0i64; friendly.len()];
	let mut api_erai_flag = vec![0i64; enemy.len()];
	let mut api_ebak_flag = vec![0i64; enemy.len()];
	let mut api_frai_flag = vec![0i64; friendly.len()];
	let mut api_fbak_flag = vec![0i64; friendly.len()];
	let mut api_fcl_flag = vec![0i64; friendly.len()];

	if total_attack_plane_count(codex, friendly) > 0 {
		let alive_targets: Vec<usize> =
			enemy.iter().enumerate().filter(|(_, s)| s.is_alive()).map(|(i, _)| i).collect();
		if !alive_targets.is_empty() {
			let target_idx = alive_targets[random.choose_index(alive_targets.len())];
			let damage = calculate_airstrike_damage(codex, friendly, &enemy[target_idx]);
			let dealt =
				enemy[target_idx].apply_damage(random, damage, target_idx);
			api_edam[target_idx] = dealt;
			api_ebak_flag[target_idx] = 1;
			api_erai_flag[target_idx] = 1;
			// Attribute damage to the ship with highest bomb power contribution
			if let Some(best_idx) = best_bomber_index(codex, friendly) {
				friendly[best_idx].damage_dealt += dealt;
			}
		}
	}

	if total_attack_plane_count(codex, enemy) > 0 {
		let alive_targets: Vec<usize> =
			friendly.iter().enumerate().filter(|(_, s)| s.is_alive()).map(|(i, _)| i).collect();
		if !alive_targets.is_empty() {
			let target_idx = alive_targets[random.choose_index(alive_targets.len())];
			let damage = calculate_airstrike_damage(codex, enemy, &friendly[target_idx]);
			let dealt =
				friendly[target_idx].apply_damage(random, damage, target_idx);
			api_fdam[target_idx] = dealt;
			api_fbak_flag[target_idx] = 1;
			api_fcl_flag[target_idx] = 1;
			api_frai_flag[target_idx] = 1;
		}
	}

	BattleKouku {
		api_plane_from: [attack_plane_from(codex, friendly), attack_plane_from(codex, enemy)],
		api_stage1: BattleKoukuStage1 {
			api_f_count: friend_planes,
			api_f_lostcount: stage1_f_lost,
			api_e_count: enemy_planes,
			api_e_lostcount: stage1_e_lost,
			api_disp_seiku: air_state.api_disp_seiku(),
			api_touch_plane: [
				first_touch_plane(codex, friendly).unwrap_or(-1),
				first_touch_plane(codex, enemy).unwrap_or(-1),
			],
		},
		api_stage2: BattleKoukuStage2 {
			api_f_count: friend_planes_after_s1,
			api_f_lostcount: stage2_f_lost,
			api_e_count: enemy_planes_after_s1,
			api_e_lostcount: stage2_e_lost,
		},
		api_stage3: BattleKoukuStage3 {
			api_frai_flag,
			api_erai_flag,
			api_fbak_flag,
			api_ebak_flag,
			api_fcl_flag,
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

/// ASW formation modifier: Diamond (3) = 1.2×, Echelon (4) = 1.1×, Line Abreast (5) = 1.3×
fn asw_formation_modifier(formation_id: i64) -> f64 {
	match formation_id {
		3 => 1.2,
		4 => 1.1,
		5 => 1.3,
		_ => 1.0,
	}
}

/// Check if a ship can perform OASW (opening anti-submarine warfare).
fn can_opening_asw(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.is_sunk() {
		return false;
	}
	let asw = ship.ship.api_taisen[0];
	let stype = ship_type(codex, ship);

	match stype {
		// DE: ASW ≥ 60
		Some(KcShipType::DE) => {
			asw >= 60
				&& (has_slotitem_type(codex, ship, KcSlotItemType3::Sonar)
					|| has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar))
		}
		// DD/CL/CT/CLT/AO: ASW ≥ 100 + sonar
		Some(
			KcShipType::DD | KcShipType::CL | KcShipType::CT | KcShipType::CLT | KcShipType::AO,
		) => {
			asw >= 100
				&& (has_slotitem_type(codex, ship, KcSlotItemType3::Sonar)
					|| has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar))
		}
		// CVL: ASW ≥ 65 + has ASW aircraft
		Some(KcShipType::CVL) => asw >= 65 && has_active_asw_aircraft(codex, ship),
		// CVB: ASW ≥ 100 + has ASW aircraft
		Some(KcShipType::CVB) => asw >= 100 && has_active_asw_aircraft(codex, ship),
		// BBV: ASW ≥ 100 + large sonar + ASW aircraft
		Some(KcShipType::BBV) => {
			asw >= 100
				&& has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar)
				&& has_active_asw_aircraft(codex, ship)
		}
		_ => false,
	}
}

/// Calculate equipment ASW from all equipped items.
fn equipment_asw_total(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
	ship.slot_items
		.iter()
		.filter_map(|si| {
			codex
				.find::<ApiMstSlotitem>(&si.api_slotitem_id)
				.ok()
				.map(|mst| mst.api_tais.max(0) as f64)
		})
		.sum()
}

/// Determine ASW equipment synergy multiplier.
fn asw_synergy_modifier(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
	let has_sonar = has_slotitem_type(codex, ship, KcSlotItemType3::Sonar);
	let has_large_sonar = has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar);
	let has_depth_charge = has_slotitem_type(codex, ship, KcSlotItemType3::DepthCharge);
	let any_sonar = has_sonar || has_large_sonar;

	// Depth charge projectors are a subset of depth charge equipment.
	// Simplified: treat all DepthCharge as both projector and charge for now.
	// Full implementation would check specific item IDs.
	let has_projector = has_depth_charge;

	if has_sonar && has_projector && has_depth_charge {
		1.4375
	} else if has_large_sonar && has_projector && has_depth_charge {
		1.265
	} else if any_sonar && has_depth_charge {
		1.15
	} else if has_projector && has_depth_charge {
		1.1
	} else {
		1.0
	}
}

/// Calculate ASW damage against a submarine target.
fn calculate_asw_damage(
	codex: &Codex,
	attacker: &BattleRuntimeShip,
	defender: &BattleRuntimeShip,
	formation_id: i64,
	engagement: EngagementType,
) -> i64 {
	let ship_asw = attacker.ship.api_taisen[0].max(0) as f64;
	let equip_asw = equipment_asw_total(codex, attacker);
	// base ASW = total ASW - equipment ASW (modernization + innate)
	let base_asw = (ship_asw - equip_asw).max(0.0);

	// Attack type bonus: +8 for aircraft ASW, +13 for depth charge
	let type_bonus = if has_active_asw_aircraft(codex, attacker) {
		8.0
	} else {
		13.0
	};

	let synergy = asw_synergy_modifier(codex, attacker);
	let raw_power = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + type_bonus) * synergy;
	let modified = raw_power * asw_formation_modifier(formation_id) * engagement.modifier();
	let capped = apply_cap(modified, 170.0) as f64;
	let armor = defender.ship.api_soukou[0].max(0) as f64 * 0.7;
	(capped - armor).floor().max(1.0) as i64
}

/// Simulate the opening ASW phase (先制対潜).
fn simulate_opening_taisen(
	codex: &Codex,
	random: &mut BattleRandom,
	friendly: &mut [BattleRuntimeShip],
	enemy: &mut [BattleRuntimeShip],
	friendly_formation_id: i64,
	enemy_formation_id: i64,
	engagement: EngagementType,
) -> Option<BattleHougeki> {
	let mut at_eflag = Vec::new();
	let mut at_list = Vec::new();
	let mut at_type = Vec::new();
	let mut df_list = Vec::new();
	let mut si_list = Vec::new();
	let mut cl_list = Vec::new();
	let mut damage = Vec::new();

	// Friendly OASW attacks
	for (idx, ship) in friendly.iter_mut().enumerate() {
		if !can_opening_asw(codex, ship) {
			continue;
		}
		let Some(target_idx) = select_submarine_target(codex, random, enemy) else {
			continue;
		};
		let raw = calculate_asw_damage(
			codex,
			ship,
			&enemy[target_idx],
			friendly_formation_id,
			engagement,
		);
		let dealt =
			enemy[target_idx].apply_damage(random, raw, target_idx);
		ship.damage_dealt += dealt;

		at_eflag.push(0);
		at_list.push(idx as i64);
		at_type.push(7); // ASW attack type
		df_list.push(vec![target_idx as i64]);
		si_list.push(day_attack_display_ids(codex, ship, true));
		cl_list.push(vec![1]);
		damage.push(vec![dealt]);
	}

	// Enemy OASW attacks
	for (idx, ship) in enemy.iter_mut().enumerate() {
		if !can_opening_asw(codex, ship) {
			continue;
		}
		let Some(target_idx) = select_submarine_target(codex, random, friendly) else {
			continue;
		};
		let raw = calculate_asw_damage(
			codex,
			ship,
			&friendly[target_idx],
			enemy_formation_id,
			engagement,
		);
		let dealt =
			friendly[target_idx].apply_damage(random, raw, target_idx);

		at_eflag.push(1);
		at_list.push(idx as i64);
		at_type.push(7);
		df_list.push(vec![target_idx as i64]);
		si_list.push(day_attack_display_ids(codex, ship, true));
		cl_list.push(vec![1]);
		damage.push(vec![dealt]);
	}

	(!at_list.is_empty()).then_some(BattleHougeki {
		api_at_eflag: at_eflag,
		api_at_list: at_list,
		api_at_type: at_type,
		api_df_list: df_list,
		api_si_list: si_list,
		api_cl_list: cl_list,
		api_damage: damage,
	})
}

/// Select a random alive submarine target.
fn select_submarine_target(
	codex: &Codex,
	random: &mut BattleRandom,
	defenders: &[BattleRuntimeShip],
) -> Option<usize> {
	let subs: Vec<usize> = defenders
		.iter()
		.enumerate()
		.filter(|(_, ship)| {
			ship.is_alive() && target_class(codex, ship) == TargetClass::Submarine
		})
		.map(|(idx, _)| idx)
		.collect();

	if subs.is_empty() {
		return None;
	}
	Some(subs[random.choose_index(subs.len())])
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

fn slotitem_mst<'a>(codex: &'a Codex, slot_item: &'a KcApiSlotItem) -> Option<&'a ApiMstSlotitem> {
	codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()
}

fn is_day_surface_display_type(slot_type: KcSlotItemType3) -> bool {
	DAY_SURFACE_DISPLAY_TYPES.contains(&slot_type)
}

fn is_asw_display_slotitem(codex: &Codex, slot_item: &KcApiSlotItem) -> bool {
	let Some(mst) = slotitem_mst(codex, slot_item) else {
		return false;
	};
	let Some(slot_type) = KcSlotItemType3::n(mst.api_type[2]) else {
		return false;
	};

	ASW_DISPLAY_TYPES.contains(&slot_type)
		|| (slot_type == KcSlotItemType3::CarrierBasedTorpedoBomber && mst.api_tais > 0)
}

fn collect_asw_display_ids(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<i64> {
	ship.slot_items
		.iter()
		.filter(|slot_item| is_asw_display_slotitem(codex, slot_item))
		.map(|slot_item| slot_item.api_slotitem_id)
		.collect()
}

fn is_night_main_gun_type(slot_type: KcSlotItemType3) -> bool {
	NIGHT_MAIN_GUN_TYPES.contains(&slot_type)
}

fn is_night_secondary_gun_type(slot_type: KcSlotItemType3) -> bool {
	NIGHT_SECONDARY_GUN_TYPES.contains(&slot_type)
}

fn is_night_torpedo_type(slot_type: KcSlotItemType3) -> bool {
	NIGHT_TORPEDO_TYPES.contains(&slot_type)
}

fn is_radar_type(slot_type: KcSlotItemType3) -> bool {
	RADAR_DISPLAY_TYPES.contains(&slot_type)
}

fn collect_matching_slot_ids(
	codex: &Codex,
	ship: &BattleRuntimeShip,
	matcher: impl Fn(KcSlotItemType3, &ApiMstSlotitem) -> bool,
) -> Vec<i64> {
	ship.slot_items
		.iter()
		.filter_map(|slot_item| {
			let mst = slotitem_mst(codex, slot_item)?;
			let slot_type = KcSlotItemType3::n(mst.api_type[2])?;
			matcher(slot_type, mst).then_some(slot_item.api_slotitem_id)
		})
		.collect()
}

fn first_or_default(ids: Vec<i64>) -> Vec<i64> {
	if ids.is_empty() {
		vec![-1]
	} else {
		vec![ids[0]]
	}
}

fn extend_limit(target: &mut Vec<i64>, source: &[i64], limit: usize) {
	for id in source {
		if target.len() >= limit {
			break;
		}
		target.push(*id);
	}
}

fn day_attack_display_ids(
	codex: &Codex,
	ship: &BattleRuntimeShip,
	is_submarine_target: bool,
) -> Vec<i64> {
	if is_submarine_target {
		let asw_ids = collect_asw_display_ids(codex, ship);
		if !asw_ids.is_empty() {
			return first_or_default(asw_ids);
		}
	}

	let surface_ids = collect_matching_slot_ids(codex, ship, |slot_type, _mst| {
		is_day_surface_display_type(slot_type)
	});
	first_or_default(surface_ids)
}

fn night_attack_display_ids(
	codex: &Codex,
	ship: &BattleRuntimeShip,
	attack_type: NightAttackType,
) -> Vec<i64> {
	let main_guns =
		collect_matching_slot_ids(codex, ship, |slot_type, _mst| is_night_main_gun_type(slot_type));
	let torpedoes =
		collect_matching_slot_ids(codex, ship, |slot_type, _mst| is_night_torpedo_type(slot_type));
	let secondary_guns = collect_matching_slot_ids(codex, ship, |slot_type, _mst| {
		is_night_secondary_gun_type(slot_type)
	});
	let radars = collect_matching_slot_ids(codex, ship, |slot_type, _mst| is_radar_type(slot_type));
	let surface_ids = collect_matching_slot_ids(codex, ship, |slot_type, _mst| {
		is_day_surface_display_type(slot_type)
	});

	let mut ids = Vec::new();
	match attack_type {
		NightAttackType::MainMainMain => extend_limit(&mut ids, &main_guns, 3),
		NightAttackType::MainMainSec => {
			extend_limit(&mut ids, &main_guns, 2);
			extend_limit(&mut ids, &secondary_guns, 3);
		}
		NightAttackType::MainTorpRadar => {
			extend_limit(&mut ids, &main_guns, 1);
			extend_limit(&mut ids, &torpedoes, 2);
			extend_limit(&mut ids, &radars, 3);
		}
		NightAttackType::TorpTorpTorp => extend_limit(&mut ids, &torpedoes, 3),
		NightAttackType::DoubleAttack => extend_limit(&mut ids, &surface_ids, 2),
		NightAttackType::Normal => extend_limit(&mut ids, &surface_ids, 1),
	}

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
	ships.iter().any(|ship| ship.is_alive())
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

/// Find the ship index with the highest total bombing power (for damage attribution).
fn best_bomber_index(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<usize> {
	ships
		.iter()
		.enumerate()
		.map(|(idx, ship)| {
			let power: f64 = ship
				.slot_items
				.iter()
				.zip(ship.ship.api_onslot)
				.filter_map(|(si, onslot)| {
					if onslot <= 0 {
						return None;
					}
					let mst = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok()?;
					if !is_airstrike_attack_type(mst.api_type[2]) {
						return None;
					}
					let is_torpedo = KcSlotItemType3::n(mst.api_type[2])
						== Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
					let stat = if is_torpedo {
						mst.api_raig.max(0) as f64
					} else {
						mst.api_baku.max(0) as f64
					};
					Some(stat * (onslot as f64).sqrt())
				})
				.sum();
			(idx, power)
		})
		.filter(|(_, power)| *power > 0.0)
		.max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
		.map(|(idx, _)| idx)
}

fn ship_type(codex: &Codex, ship: &BattleRuntimeShip) -> Option<KcShipType> {
	KcShipType::n(codex.get_ship_type(ship.ship.api_ship_id) as i32)
}

fn target_class(codex: &Codex, ship: &BattleRuntimeShip) -> TargetClass {
	match ship_type(codex, ship) {
		Some(KcShipType::SS | KcShipType::SSV) => TargetClass::Submarine,
		_ => TargetClass::Surface,
	}
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
	if ship.is_sunk() || ship.ship.api_raisou[0] <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(KcShipType::CLT | KcShipType::SS | KcShipType::SSV) => true,
		_ => has_slotitem_type(codex, ship, KcSlotItemType3::SpecialSubmarineVessel),
	}
}

fn can_closing_torpedo_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.is_sunk() || ship.ship.api_raisou[0] <= 0 {
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
	if ship.is_sunk() {
		return false;
	}

	match ship_type(codex, ship) {
		Some(KcShipType::SS | KcShipType::SSV) => false,
		Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
			total_attack_plane_count(codex, std::slice::from_ref(ship)) > 0
		}
		_ => true,
	}
}

fn can_attack_night_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.is_sunk() {
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
		Some(KcShipType::SS | KcShipType::SSV) => false,
		_ => true,
	}
}

fn attack_capability_for_phase(
	codex: &Codex,
	ship: &BattleRuntimeShip,
	phase: BattlePhase,
) -> AttackCapability {
	match phase {
		BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => {
			if ship.is_alive() && ship.ship.api_raisou[0] > 0 {
				AttackCapability::SurfaceOnly
			} else {
				AttackCapability::CannotAttack
			}
		}
		BattlePhase::DayShelling => {
			if !can_shell_day_ship(codex, ship) {
				AttackCapability::CannotAttack
			} else if can_attack_submarine_day_shelling(codex, ship) {
				AttackCapability::BothPreferSubmarine
			} else {
				AttackCapability::SurfaceOnly
			}
		}
		BattlePhase::NightShelling => {
			if !can_attack_night_ship(codex, ship) {
				AttackCapability::CannotAttack
			} else if can_attack_submarine_night_shelling(codex, ship) {
				AttackCapability::BothPreferSubmarine
			} else {
				AttackCapability::SurfaceOnly
			}
		}
	}
}

fn can_attack_submarine_day_shelling(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.is_sunk() || ship.ship.api_taisen[0] <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(
			KcShipType::DE
			| KcShipType::DD
			| KcShipType::CL
			| KcShipType::CLT
			| KcShipType::CT
			| KcShipType::AO,
		) => true,
		Some(
			KcShipType::BBV | KcShipType::CAV | KcShipType::AV | KcShipType::LHA | KcShipType::CVL,
		) => has_active_asw_aircraft(codex, ship),
		_ => false,
	}
}

fn can_attack_submarine_night_shelling(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	if ship.is_sunk() || ship.ship.api_taisen[0] <= 0 {
		return false;
	}

	match ship_type(codex, ship) {
		Some(
			KcShipType::DE
			| KcShipType::DD
			| KcShipType::CL
			| KcShipType::CLT
			| KcShipType::CT
			| KcShipType::AO,
		) => true,
		Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
			can_attack_night_ship(codex, ship) && has_active_asw_aircraft(codex, ship)
		}
		_ => false,
	}
}

fn has_active_asw_aircraft(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
		let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok() else {
			return false;
		};
		matches!(
			KcSlotItemType3::n(mst.api_type[2]),
			Some(
				KcSlotItemType3::AutoGyro
					| KcSlotItemType3::AntiSubmarinePatrol
					| KcSlotItemType3::SeaBasedBomber
					| KcSlotItemType3::LargeFlyingBoat
			)
		) && onslot > 0
	})
}

fn select_random_target_index(
	codex: &Codex,
	random: &mut BattleRandom,
	attacker: &BattleRuntimeShip,
	defenders: &[BattleRuntimeShip],
	phase: BattlePhase,
) -> Option<usize> {
	let alive_targets = defenders
		.iter()
		.enumerate()
		.filter(|(_, ship)| ship.is_alive())
		.map(|(idx, _)| idx)
		.collect::<Vec<_>>();
	if alive_targets.is_empty() {
		return None;
	}

	let surface_targets = alive_targets
		.iter()
		.copied()
		.filter(|idx| target_class(codex, &defenders[*idx]) == TargetClass::Surface)
		.collect::<Vec<_>>();
	let submarine_targets = alive_targets
		.iter()
		.copied()
		.filter(|idx| target_class(codex, &defenders[*idx]) == TargetClass::Submarine)
		.collect::<Vec<_>>();

	let candidates = match attack_capability_for_phase(codex, attacker, phase) {
		AttackCapability::CannotAttack => return None,
		AttackCapability::SurfaceOnly => surface_targets,
		AttackCapability::BothPreferSubmarine => {
			if submarine_targets.is_empty() {
				surface_targets
			} else {
				submarine_targets
			}
		}
	};
	if candidates.is_empty() {
		return None;
	}

	Some(candidates[random.choose_index(candidates.len())])
}

fn calculate_scratch_damage(random: &mut BattleRandom, current_hp: i64) -> i64 {
	random.roll_scratch_damage(current_hp).min(current_hp.max(1))
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
	let enemy_remaining_hp: i64 = enemy.iter().map(|ship| ship.hp().max(0)).sum();
	let friend_total_hp: i64 = friendly.iter().map(|ship| ship.ship.api_maxhp).sum();
	let friend_remaining_hp: i64 = friendly.iter().map(|ship| ship.hp().max(0)).sum();
	let enemy_all_sunk = enemy.iter().all(|ship| ship.is_sunk());
	let friend_all_sunk = friendly.iter().all(|ship| ship.is_sunk());
	let friend_sunk_count = friendly.iter().filter(|ship| ship.is_sunk()).count();
	let friend_count = friendly.len();
	let enemy_damage_rate =
		(enemy_total_hp - enemy_remaining_hp) as f64 / enemy_total_hp.max(1) as f64;
	let friend_damage_rate =
		(friend_total_hp - friend_remaining_hp) as f64 / friend_total_hp.max(1) as f64;

	let rank = if friend_all_sunk {
		KcSortieResultRank::E
	} else if enemy_all_sunk && friend_sunk_count == 0 {
		KcSortieResultRank::S
	} else if enemy_all_sunk {
		// All enemy sunk but we lost ships → downgrade to A
		KcSortieResultRank::A
	} else if friend_sunk_count * 2 >= friend_count && friend_count > 1 {
		// Half or more friendly ships sunk → D
		KcSortieResultRank::D
	} else if enemy_damage_rate >= 0.7 {
		if friend_sunk_count > 0 {
			KcSortieResultRank::B
		} else {
			KcSortieResultRank::A
		}
	} else if enemy_damage_rate > friend_damage_rate {
		KcSortieResultRank::B
	} else {
		KcSortieResultRank::C
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

/// Night battle special attack (cut-in / double attack) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NightAttackType {
	Normal,
	DoubleAttack,  // 連撃: 2 hits × 1.2×
	MainMainMain,  // 主主主CI: 1 hit × 2.0×
	MainMainSec,   // 主主副CI: 1 hit × 1.75×
	TorpTorpTorp,  // 鱼鱼鱼CI: 2 hits × 1.3×
	MainTorpRadar, // 主鱼電CI: 1 hit × 1.625×
}

impl NightAttackType {
	fn api_sp_list(self) -> i64 {
		match self {
			Self::Normal => 0,
			Self::DoubleAttack => 1,
			Self::MainMainMain => 2,
			Self::MainMainSec => 3,
			Self::TorpTorpTorp => 4,
			Self::MainTorpRadar => 5,
		}
	}

	fn damage_multiplier(self) -> f64 {
		match self {
			Self::Normal => 1.0,
			Self::DoubleAttack => 1.2,
			Self::MainMainMain => 2.0,
			Self::MainMainSec => 1.75,
			Self::TorpTorpTorp => 1.3,
			Self::MainTorpRadar => 1.625,
		}
	}

	fn hit_count(self) -> usize {
		match self {
			Self::Normal | Self::MainMainMain | Self::MainMainSec | Self::MainTorpRadar => 1,
			Self::DoubleAttack | Self::TorpTorpTorp => 2,
		}
	}

	fn ci_coefficient(self) -> f64 {
		match self {
			Self::TorpTorpTorp => 122.0,
			Self::MainTorpRadar => 115.0,
			Self::MainMainSec => 130.0,
			Self::MainMainMain => 140.0,
			Self::DoubleAttack | Self::Normal => 0.0,
		}
	}
}

fn count_equipment_type(codex: &Codex, ship: &BattleRuntimeShip, wanted: KcSlotItemType3) -> usize {
	ship.slot_items
		.iter()
		.filter(|si| {
			codex
				.find::<ApiMstSlotitem>(&si.api_slotitem_id)
				.ok()
				.and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
				== Some(wanted)
		})
		.count()
}

fn is_main_gun_type(t: KcSlotItemType3) -> bool {
	matches!(
		t,
		KcSlotItemType3::SmallCaliberMainGun
			| KcSlotItemType3::MediumCaliberMainGun
			| KcSlotItemType3::LargeCaliberMainGun
			| KcSlotItemType3::LargeCaliberMainGun2
	)
}

fn count_main_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
	ship.slot_items
		.iter()
		.filter(|si| {
			codex
				.find::<ApiMstSlotitem>(&si.api_slotitem_id)
				.ok()
				.and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
				.is_some_and(is_main_gun_type)
		})
		.count()
}

fn count_secondary_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
	ship.slot_items
		.iter()
		.filter(|si| {
			codex
				.find::<ApiMstSlotitem>(&si.api_slotitem_id)
				.ok()
				.and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
				.is_some_and(|t| {
					matches!(t, KcSlotItemType3::SecondaryGun | KcSlotItemType3::SecondaryGun2)
				})
		})
		.count()
}

fn has_radar(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
	has_slotitem_type(codex, ship, KcSlotItemType3::SmallRadar)
		|| has_slotitem_type(codex, ship, KcSlotItemType3::LargeRadar)
		|| has_slotitem_type(codex, ship, KcSlotItemType3::LargeRadar2)
}

/// Detect the best night attack type from equipment loadout.
fn detect_night_attack_type(codex: &Codex, ship: &BattleRuntimeShip) -> NightAttackType {
	let main_guns = count_main_guns(codex, ship);
	let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
		+ count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
	let sec_guns = count_secondary_guns(codex, ship);
	let has_radar = has_radar(codex, ship);

	// CI priority (highest first): 主主主 > 主主副 > 主鱼電 > 鱼鱼鱼 > 連撃
	if main_guns >= 3 {
		return NightAttackType::MainMainMain;
	}
	if main_guns >= 2 && sec_guns >= 1 {
		return NightAttackType::MainMainSec;
	}
	if main_guns >= 1 && torps >= 1 && has_radar {
		return NightAttackType::MainTorpRadar;
	}
	if torps >= 2 {
		return NightAttackType::TorpTorpTorp;
	}
	// Double attack: 2+ different weapon categories (main + secondary, main + torp, etc.)
	if (main_guns >= 2) || (main_guns >= 1 && sec_guns >= 1) || (main_guns >= 1 && torps >= 1) {
		return NightAttackType::DoubleAttack;
	}
	NightAttackType::Normal
}

/// Calculate night CI trigger rate.
fn night_ci_trigger_rate(
	ship: &BattleRuntimeShip,
	ci_type: NightAttackType,
	is_flagship: bool,
) -> f64 {
	let coefficient = ci_type.ci_coefficient();
	if coefficient <= 0.0 {
		return if ci_type == NightAttackType::DoubleAttack {
			0.99
		} else {
			0.0
		};
	}

	let luck = ship.ship.api_lucky[0].max(0) as f64;
	let level = ship.ship.api_lv.max(1) as f64;

	let ci_value = if luck < 50.0 {
		15.0 + luck + (0.75 * level.sqrt()).floor()
	} else {
		65.0 + (luck - 50.0).sqrt() + (0.8 * level.sqrt()).floor()
	};

	let modifier = if is_flagship {
		15.0
	} else {
		0.0
	};
	// Chuuha modifier omitted for simplicity (would need HP check)

	let total = ci_value + modifier;
	(total / coefficient).clamp(0.0, 1.0)
}

/// Resolve night attack type: detect CI from equipment, then roll trigger.
fn resolve_night_attack(
	codex: &Codex,
	random: &mut BattleRandom,
	ship: &BattleRuntimeShip,
	is_flagship: bool,
	is_submarine_target: bool,
) -> NightAttackType {
	if is_submarine_target {
		return NightAttackType::Normal;
	}
	let detected = detect_night_attack_type(codex, ship);
	if detected == NightAttackType::Normal {
		return NightAttackType::Normal;
	}
	if detected == NightAttackType::DoubleAttack {
		// Double attack has ~99% trigger
		return NightAttackType::DoubleAttack;
	}
	// Roll CI trigger
	let rate = night_ci_trigger_rate(ship, detected, is_flagship);
	let roll = random.random_f64_range(0.0, 1.0);
	if roll < rate {
		detected
	} else {
		// Failed CI → check for double attack fallback
		let main_guns = count_main_guns(codex, ship);
		let sec_guns = count_secondary_guns(codex, ship);
		let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
			+ count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
		if (main_guns >= 2) || (main_guns >= 1 && sec_guns >= 1) || (main_guns >= 1 && torps >= 1) {
			NightAttackType::DoubleAttack
		} else {
			NightAttackType::Normal
		}
	}
}

fn simulate_night_hougeki(
	codex: &Codex,
	random: &mut BattleRandom,
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
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, enemy, BattlePhase::NightShelling)
		else {
			continue;
		};
		let is_submarine = target_class(codex, &enemy[target_idx]) == TargetClass::Submarine;
		let attack_type = resolve_night_attack(codex, random, ship, idx == 0, is_submarine);
		let hits = attack_type.hit_count();
		let multiplier = attack_type.damage_multiplier();

		let mut hit_damages = Vec::new();
		let mut hit_cls = Vec::new();
		let mut total_dealt = 0i64;

		for _ in 0..hits {
			let raw = if is_submarine {
				calculate_scratch_damage(random, enemy[target_idx].hp().max(1))
			} else {
				let base = calculate_night_damage(ship, &enemy[target_idx], engagement);
				(base as f64 * multiplier).floor() as i64
			};
			let dealt =
				enemy[target_idx].apply_damage(random, raw, target_idx);
			total_dealt += dealt;
			hit_damages.push(dealt);
			hit_cls.push(1i64);
		}
		ship.damage_dealt += total_dealt;

		at_eflag.push(0);
		at_list.push(idx as i64);
		n_mother_list.push(0);
		df_list.push(vec![target_idx as i64; hits]);
		si_list.push(night_attack_display_ids(codex, ship, attack_type));
		cl_list.push(hit_cls);
		sp_list.push(attack_type.api_sp_list());
		damage.push(hit_damages);
	}

	for (idx, ship) in enemy.iter_mut().enumerate() {
		if !can_attack_night_ship(codex, ship) {
			continue;
		}
		let Some(target_idx) =
			select_random_target_index(codex, random, ship, friendly, BattlePhase::NightShelling)
		else {
			continue;
		};
		let is_submarine = target_class(codex, &friendly[target_idx]) == TargetClass::Submarine;
		let attack_type = resolve_night_attack(codex, random, ship, idx == 0, is_submarine);
		let hits = attack_type.hit_count();
		let multiplier = attack_type.damage_multiplier();

		let mut hit_damages = Vec::new();
		let mut hit_cls = Vec::new();

		for _ in 0..hits {
			let raw = if is_submarine {
				calculate_scratch_damage(random, friendly[target_idx].hp().max(1))
			} else {
				let base = calculate_night_damage(ship, &friendly[target_idx], engagement);
				(base as f64 * multiplier).floor() as i64
			};
			let dealt =
				friendly[target_idx].apply_damage(random, raw, target_idx);
			hit_damages.push(dealt);
			hit_cls.push(1i64);
		}

		at_eflag.push(1);
		at_list.push(idx as i64);
		n_mother_list.push(0);
		df_list.push(vec![target_idx as i64; hits]);
		si_list.push(night_attack_display_ids(codex, ship, attack_type));
		cl_list.push(hit_cls);
		sp_list.push(attack_type.api_sp_list());
		damage.push(hit_damages);
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

/// Post-simulation integrity check: verifies that protected friendly ships
/// (non-taiha at entry + flagship) have HP ≥ 1.
/// Panics in debug builds if a protected ship was incorrectly sunk.
fn verify_protected_ships_alive(ships: &[BattleRuntimeShip]) {
	for (idx, ship) in ships.iter().enumerate() {
		if !ship.is_friendly || !ship.is_sortie {
			continue;
		}
		let was_taiha = ship.entry_hp * 4 <= ship.ship.api_maxhp;
		let is_protected = idx == 0 || !was_taiha;
		if is_protected {
			debug_assert!(
				ship.hp() >= 1,
				"BUG: protected ship at index {} has hp={}, entry_hp={}, maxhp={}",
				idx,
				ship.hp(),
				ship.entry_hp,
				ship.ship.api_maxhp
			);
		}
	}
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

	fn slotitem_mst_id_by_name(codex: &Codex, name: &str) -> i64 {
		codex
			.manifest
			.api_mst_slotitem
			.iter()
			.find(|mst| mst.api_name == name)
			.map(|mst| mst.api_id)
			.unwrap()
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
				battle_type: BattleType::Normal,
				is_sortie: true,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![friend],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
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
				battle_type: BattleType::Normal,
				is_sortie: false,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![carrier],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
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
				battle_type: BattleType::Normal,
				is_sortie: false,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![dd, clt],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
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
				battle_type: BattleType::Normal,
				is_sortie: false,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![carrier, bb],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
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

	#[test]
	fn day_shelling_destroyer_prefers_submarine_targets() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

		let attacker = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50));
		let defenders = vec![
			BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50)),
			BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
		];
		let mut random = BattleRandom::new(Some(7));

		let target_idx = select_random_target_index(
			&codex,
			&mut random,
			&attacker,
			&defenders,
			BattlePhase::DayShelling,
		)
		.unwrap();

		assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::Submarine);
	}

	#[test]
	fn day_shelling_battleship_ignores_submarine_targets() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

		let attacker = BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50));
		let defenders = vec![
			BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
			BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
		];
		let mut random = BattleRandom::new(Some(7));

		let target_idx = select_random_target_index(
			&codex,
			&mut random,
			&attacker,
			&defenders,
			BattlePhase::DayShelling,
		)
		.unwrap();

		assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::Surface);
	}

	#[test]
	fn day_shelling_display_ids_skip_non_attack_equipment_like_night_recon() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let night_recon_mst_id = slotitem_mst_id_by_name(&codex, "九八式水上偵察機(夜偵)");
		let main_gun_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

		let mut ship = sample_ship(&codex, bb_mst, 50);
		ship.slot_items =
			vec![slotitem_with_mst_id(night_recon_mst_id), slotitem_with_mst_id(main_gun_mst_id)];
		let runtime_ship = BattleRuntimeShip::from(ship);

		assert_eq!(day_attack_display_ids(&codex, &runtime_ship, false), vec![main_gun_mst_id]);
	}

	#[test]
	fn day_asw_display_ids_ignore_night_recon_when_valid_asw_equipment_exists() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
		let night_recon_mst_id = slotitem_mst_id_by_name(&codex, "九八式水上偵察機(夜偵)");
		let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

		let mut ship = sample_ship(&codex, bbv_mst, 50);
		ship.slot_items =
			vec![slotitem_with_mst_id(night_recon_mst_id), slotitem_with_mst_id(sonar_mst_id)];
		let runtime_ship = BattleRuntimeShip::from(ship);

		assert_eq!(day_attack_display_ids(&codex, &runtime_ship, true), vec![sonar_mst_id]);
	}

	#[test]
	fn torpedo_targeting_ignores_submarines() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

		let attacker = BattleRuntimeShip::from(sample_ship(&codex, clt_mst, 50));
		let mixed_defenders = vec![
			BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
			BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
		];
		let submarine_only = vec![BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50))];
		let mut random = BattleRandom::new(Some(11));

		let target_idx = select_random_target_index(
			&codex,
			&mut random,
			&attacker,
			&mixed_defenders,
			BattlePhase::ClosingTorpedo,
		)
		.unwrap();
		assert_eq!(target_class(&codex, &mixed_defenders[target_idx]), TargetClass::Surface);
		assert!(
			select_random_target_index(
				&codex,
				&mut random,
				&attacker,
				&submarine_only,
				BattlePhase::OpeningTorpedo,
			)
			.is_none()
		);
	}

	#[test]
	fn night_shelling_against_submarines_is_scratch_damage() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

		let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50))];
		let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50))];
		let enemy_hp = enemy[0].hp();
		let mut random = BattleRandom::new(Some(3));

		let hougeki = simulate_night_hougeki(
			&codex,
			&mut random,
			&mut friendly,
			&mut enemy,
			1,
			1,
			EngagementType::SameCourse,
		)
		.unwrap();

		assert_eq!(hougeki.api_df_list[0], vec![0]);
		assert!(hougeki.api_damage[0][0] >= 1);
		assert!(hougeki.api_damage[0][0] < enemy_hp);
		assert_eq!(enemy[0].hp(), enemy_hp - hougeki.api_damage[0][0]);
	}

	#[test]
	fn air_state_supremacy_when_friendly_triples_enemy() {
		assert_eq!(AirState::from_power(300, 100), AirState::Supremacy);
		assert_eq!(AirState::from_power(300, 0), AirState::Supremacy);
		assert_eq!(AirState::from_power(301, 100), AirState::Supremacy);
	}

	#[test]
	fn air_state_superiority_when_friendly_exceeds_1_5x() {
		assert_eq!(AirState::from_power(150, 100), AirState::Superiority);
		assert_eq!(AirState::from_power(200, 100), AirState::Superiority);
		// 299 < 300 so not supremacy, but 2*299=598 >= 3*100=300 so superiority
		assert_eq!(AirState::from_power(299, 100), AirState::Superiority);
	}

	#[test]
	fn air_state_parity_in_middle_range() {
		assert_eq!(AirState::from_power(0, 0), AirState::Parity);
		assert_eq!(AirState::from_power(100, 100), AirState::Parity);
		assert_eq!(AirState::from_power(149, 100), AirState::Parity);
		assert_eq!(AirState::from_power(100, 149), AirState::Parity);
	}

	#[test]
	fn air_state_denial_when_enemy_exceeds_1_5x() {
		assert_eq!(AirState::from_power(100, 150), AirState::Denial);
		assert_eq!(AirState::from_power(100, 200), AirState::Denial);
	}

	#[test]
	fn air_state_incapability_when_enemy_triples_friendly() {
		assert_eq!(AirState::from_power(100, 300), AirState::Incapability);
		assert_eq!(AirState::from_power(100, 301), AirState::Incapability);
		assert_eq!(AirState::from_power(0, 100), AirState::Incapability);
	}

	#[test]
	fn air_state_api_disp_seiku_values() {
		assert_eq!(AirState::Supremacy.api_disp_seiku(), 1);
		assert_eq!(AirState::Superiority.api_disp_seiku(), 2);
		assert_eq!(AirState::Parity.api_disp_seiku(), 0);
		assert_eq!(AirState::Denial.api_disp_seiku(), 3);
		assert_eq!(AirState::Incapability.api_disp_seiku(), 4);
	}

	#[test]
	fn fighter_power_calculates_from_equipment_aa_and_slot_count() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let fighter_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
		let fighter_mst = codex.manifest.find_slotitem(fighter_mst_id).unwrap();
		let aa = fighter_mst.api_tyku;

		let mut ship_input =
			sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 50);
		ship_input.ship.api_onslot = [18, 0, 0, 0, 0];
		ship_input.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];

		let ships = vec![BattleRuntimeShip::from(ship_input)];
		let power = calculate_fighter_power(&codex, &ships);
		let expected = (aa as f64 * (18.0_f64).sqrt()).floor() as i64;
		assert_eq!(power, expected);
	}

	#[test]
	fn kouku_stage1_reports_nonzero_losses_when_planes_present() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let mut friend = sample_ship(&codex, cvl_mst, 50);
		friend.ship.api_soukou[0] = 200;
		friend.ship.api_nowhp = 200;
		friend.ship.api_maxhp = 200;

		let mut enemy = sample_ship(&codex, cvl_mst, 50);
		enemy.ship.api_soukou[0] = 200;
		enemy.ship.api_nowhp = 200;
		enemy.ship.api_maxhp = 200;

		let mut friendly = vec![BattleRuntimeShip::from(friend)];
		let mut enemies = vec![BattleRuntimeShip::from(enemy)];
		let mut random = BattleRandom::new(Some(42));

		let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut random);

		assert!(kouku.api_stage1.api_f_count > 0);
		assert!(kouku.api_stage1.api_e_count > 0);
		let total_f_lost = kouku.api_stage1.api_f_lostcount + kouku.api_stage2.api_f_lostcount;
		let total_e_lost = kouku.api_stage1.api_e_lostcount + kouku.api_stage2.api_e_lostcount;
		// With seed 42 and two CVLs, at least some losses should occur
		assert!(total_f_lost + total_e_lost > 0 || kouku.api_stage1.api_f_count == 0);
	}

	#[test]
	fn kouku_does_not_wipe_all_enemy_planes_unconditionally() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

		let mut friend = sample_ship(&codex, bb_mst, 50);
		friend.ship.api_soukou[0] = 200;
		friend.ship.api_nowhp = 200;
		friend.ship.api_maxhp = 200;
		friend.ship.api_taiku[0] = 10;

		let mut enemy = sample_ship(&codex, cvl_mst, 50);
		enemy.ship.api_soukou[0] = 200;
		enemy.ship.api_nowhp = 200;
		enemy.ship.api_maxhp = 200;

		let mut friendly = vec![BattleRuntimeShip::from(friend)];
		let mut enemies = vec![BattleRuntimeShip::from(enemy)];
		let mut random = BattleRandom::new(Some(42));

		let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut random);

		// The old bug wiped ALL enemy planes. Now with proportional losses,
		// a BB with low AA should NOT annihilate all enemy carrier planes.
		let remaining_enemy_planes = total_plane_count(&codex, &enemies);
		assert!(remaining_enemy_planes > 0, "enemy planes should not be fully wiped");
		// Stage 2 enemy losses should be bounded by friendly AA contribution
		assert!(kouku.api_stage2.api_e_lostcount < kouku.api_stage2.api_e_count);
	}

	#[test]
	fn kouku_air_state_reflects_fighter_power_balance() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

		// CVL (with planes) vs DD (no planes) → supremacy
		let mut friend = sample_ship(&codex, cvl_mst, 50);
		// Ensure the CVL has fighter planes by equipping a fighter in a slot with planes
		let fighter_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
		friend.ship.api_onslot = [24, 0, 0, 0, 0];
		friend.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];
		friend.ship.api_soukou[0] = 200;
		friend.ship.api_nowhp = 200;
		friend.ship.api_maxhp = 200;

		let mut enemy = sample_ship(&codex, dd_mst, 50);
		enemy.ship.api_soukou[0] = 200;
		enemy.ship.api_nowhp = 200;
		enemy.ship.api_maxhp = 200;

		let friendly_fp =
			calculate_fighter_power(&codex, &[BattleRuntimeShip::from(friend.clone())]);
		assert!(friendly_fp > 0, "CVL with fighter should have positive fighter power");

		let mut friendly = vec![BattleRuntimeShip::from(friend)];
		let mut enemies = vec![BattleRuntimeShip::from(enemy)];
		let mut random = BattleRandom::new(Some(42));

		let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut random);
		assert_eq!(kouku.api_stage1.api_disp_seiku, 1); // supremacy
	}

	#[test]
	fn asw_formation_modifier_diamond_and_line_abreast() {
		assert!((asw_formation_modifier(3) - 1.2).abs() < f64::EPSILON);
		assert!((asw_formation_modifier(4) - 1.1).abs() < f64::EPSILON);
		assert!((asw_formation_modifier(5) - 1.3).abs() < f64::EPSILON);
		assert!((asw_formation_modifier(1) - 1.0).abs() < f64::EPSILON);
		assert!((asw_formation_modifier(2) - 1.0).abs() < f64::EPSILON);
	}

	#[test]
	fn oasw_requires_sufficient_asw_and_sonar() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

		// DD with ASW 100 + sonar → can OASW
		let mut ship = sample_ship(&codex, dd_mst, 99);
		ship.ship.api_taisen[0] = 100;
		ship.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
		let rt = BattleRuntimeShip::from(ship);
		assert!(can_opening_asw(&codex, &rt));

		// DD with ASW 99 + sonar → cannot OASW
		let mut ship2 = sample_ship(&codex, dd_mst, 99);
		ship2.ship.api_taisen[0] = 99;
		ship2.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
		let rt2 = BattleRuntimeShip::from(ship2);
		assert!(!can_opening_asw(&codex, &rt2));

		// DD with ASW 100 but no sonar → cannot OASW
		let mut ship3 = sample_ship(&codex, dd_mst, 99);
		ship3.ship.api_taisen[0] = 100;
		ship3.slot_items = vec![];
		let rt3 = BattleRuntimeShip::from(ship3);
		assert!(!can_opening_asw(&codex, &rt3));
	}

	#[test]
	fn oasw_de_threshold_is_60() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let de_mst = first_ship_mst_by_type(&codex, KcShipType::DE);
		let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

		let mut ship = sample_ship(&codex, de_mst, 50);
		ship.ship.api_taisen[0] = 60;
		ship.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
		let rt = BattleRuntimeShip::from(ship);
		assert!(can_opening_asw(&codex, &rt));

		let mut ship2 = sample_ship(&codex, de_mst, 50);
		ship2.ship.api_taisen[0] = 59;
		ship2.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
		let rt2 = BattleRuntimeShip::from(ship2);
		assert!(!can_opening_asw(&codex, &rt2));
	}

	#[test]
	fn asw_damage_formula_uses_sqrt_base_and_equipment() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
		let dc_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::DepthCharge);
		let dc_mst = codex.manifest.find_slotitem(dc_mst_id).unwrap();
		let equip_asw = dc_mst.api_tais.max(0) as f64;

		let mut attacker_input = sample_ship(&codex, dd_mst, 50);
		attacker_input.ship.api_taisen[0] = 80;
		attacker_input.slot_items = vec![slotitem_with_mst_id(dc_mst_id)];
		let attacker = BattleRuntimeShip::from(attacker_input);

		let mut defender_input = sample_ship(&codex, ss_mst, 50);
		defender_input.ship.api_soukou[0] = 10;
		let defender = BattleRuntimeShip::from(defender_input);

		let dmg = calculate_asw_damage(
			&codex,
			&attacker,
			&defender,
			1, // line ahead
			EngagementType::SameCourse,
		);

		// Verify damage is positive and uses the ASW formula (not shelling formula)
		assert!(dmg >= 1);
		// raw_power = (√(80 - equip_asw) * 2 + √equip_asw * 1.5 + 13) * synergy
		// With a single depth charge: projector=true, dc=true → synergy = 1.1
		let base_asw = (80.0 - equip_asw).max(0.0);
		let synergy = 1.1; // single DepthCharge counts as both projector and charge
		let expected_raw = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + 13.0) * synergy;
		let expected_capped = apply_cap(expected_raw, 170.0) as f64;
		let expected_dmg = (expected_capped - 10.0 * 0.7).floor().max(1.0) as i64;
		assert_eq!(dmg, expected_dmg);
	}

	#[test]
	fn oasw_targets_submarines_only() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

		let defenders = vec![
			BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
			BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
		];
		let mut random = BattleRandom::new(Some(42));

		// Should always select index 1 (the submarine), never index 0 (the DD)
		for _ in 0..10 {
			let idx = select_submarine_target(&codex, &mut random, &defenders).unwrap();
			assert_eq!(idx, 1, "OASW should only target submarines");
		}
	}

	#[test]
	fn oasw_fires_in_day_battle_when_conditions_met() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
		let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

		let mut friend = sample_ship(&codex, dd_mst, 99);
		friend.ship.api_taisen[0] = 100;
		friend.ship.api_soukou[0] = 200;
		friend.ship.api_nowhp = 200;
		friend.ship.api_maxhp = 200;
		friend.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];

		let mut enemy = sample_ship(&codex, ss_mst, 50);
		enemy.ship.api_soukou[0] = 5;
		enemy.ship.api_nowhp = 30;
		enemy.ship.api_maxhp = 30;

		let context = BattleContext {
			mode: BattleMode::Sortie,
			battle_type: BattleType::Normal,
			is_sortie: true,
			friendly_formation_id: 1,
			enemy_formation_id: 1,
			engagement: EngagementType::SameCourse,
			friend_ships: vec![friend],
			enemy_ships: vec![enemy],
			rng_seed: Some(42),
		};

		let result = simulate_day_battle_v1(&codex, context);
		assert_eq!(result.packet.opening_taisen_flag, 1);
		assert!(result.packet.opening_taisen.is_some());

		let taisen = result.packet.opening_taisen.unwrap();
		assert_eq!(taisen.api_at_eflag, vec![0]);
		assert_eq!(taisen.api_at_type, vec![7]);
		assert!(taisen.api_damage[0][0] >= 1);
	}

	#[test]
	fn night_ci_triple_main_gun_detects_as_main_main_main() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let main_gun_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

		let mut ship = sample_ship(&codex, bb_mst, 99);
		ship.slot_items = vec![
			slotitem_with_mst_id(main_gun_mst_id),
			slotitem_with_mst_id(main_gun_mst_id),
			slotitem_with_mst_id(main_gun_mst_id),
		];
		let rt = BattleRuntimeShip::from(ship);
		let attack = detect_night_attack_type(&codex, &rt);
		assert_eq!(attack, NightAttackType::MainMainMain);
		assert!((attack.damage_multiplier() - 2.0).abs() < f64::EPSILON);
		assert_eq!(attack.hit_count(), 1);
	}

	#[test]
	fn night_ci_torpedo_torpedo_detects_as_torp_ci() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

		let mut ship = sample_ship(&codex, dd_mst, 99);
		ship.slot_items =
			vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];
		let rt = BattleRuntimeShip::from(ship);
		let attack = detect_night_attack_type(&codex, &rt);
		assert_eq!(attack, NightAttackType::TorpTorpTorp);
		assert!((attack.damage_multiplier() - 1.3).abs() < f64::EPSILON);
		assert_eq!(attack.hit_count(), 2);
	}

	#[test]
	fn night_ci_main_main_secondary_detects_correctly() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let main_gun_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
		let sec_gun_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);

		let mut ship = sample_ship(&codex, bb_mst, 99);
		ship.slot_items = vec![
			slotitem_with_mst_id(main_gun_mst_id),
			slotitem_with_mst_id(main_gun_mst_id),
			slotitem_with_mst_id(sec_gun_mst_id),
		];
		let rt = BattleRuntimeShip::from(ship);
		let attack = detect_night_attack_type(&codex, &rt);
		assert_eq!(attack, NightAttackType::MainMainSec);
		assert!((attack.damage_multiplier() - 1.75).abs() < f64::EPSILON);
	}

	#[test]
	fn night_double_attack_with_main_and_torpedo() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let main_gun_mst_id =
			first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
		let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

		let mut ship = sample_ship(&codex, dd_mst, 99);
		ship.slot_items =
			vec![slotitem_with_mst_id(main_gun_mst_id), slotitem_with_mst_id(torp_mst_id)];
		let rt = BattleRuntimeShip::from(ship);
		// main×1 + torp×1 + no radar → no CI, but qualifies for double attack
		let attack = detect_night_attack_type(&codex, &rt);
		assert_eq!(attack, NightAttackType::DoubleAttack);
		assert_eq!(attack.hit_count(), 2);
		assert!((attack.damage_multiplier() - 1.2).abs() < f64::EPSILON);
	}

	#[test]
	fn night_ci_trigger_rate_increases_with_luck() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

		let mut low_luck_ship = sample_ship(&codex, dd_mst, 99);
		low_luck_ship.ship.api_lucky = [10, 99];
		let rt_low = BattleRuntimeShip::from(low_luck_ship);

		let mut high_luck_ship = sample_ship(&codex, dd_mst, 99);
		high_luck_ship.ship.api_lucky = [80, 99];
		let rt_high = BattleRuntimeShip::from(high_luck_ship);

		let rate_low = night_ci_trigger_rate(&rt_low, NightAttackType::TorpTorpTorp, false);
		let rate_high = night_ci_trigger_rate(&rt_high, NightAttackType::TorpTorpTorp, false);
		assert!(
			rate_high > rate_low,
			"higher luck should give higher CI rate: {rate_high} > {rate_low}"
		);
	}

	#[test]
	fn night_battle_sp_list_nonzero_for_ci_ship() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

		// Give DD high stats + 2 torpedoes + high luck for guaranteed CI
		let mut friend = sample_ship(&codex, dd_mst, 99);
		friend.ship.api_lucky = [90, 99];
		friend.ship.api_karyoku[0] = 150;
		friend.ship.api_raisou[0] = 200;
		friend.ship.api_soukou[0] = 200;
		friend.ship.api_nowhp = 200;
		friend.ship.api_maxhp = 200;
		friend.slot_items =
			vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];

		let mut enemy_ship = sample_ship(&codex, dd_mst, 50);
		enemy_ship.ship.api_soukou[0] = 10;
		enemy_ship.ship.api_nowhp = 500;
		enemy_ship.ship.api_maxhp = 500;
		enemy_ship.ship.api_karyoku[0] = 1;

		let mut friendly = vec![BattleRuntimeShip::from(friend)];
		let mut enemies = vec![BattleRuntimeShip::from(enemy_ship)];
		let mut random = BattleRandom::new(Some(42));

		let hougeki = simulate_night_hougeki(
			&codex,
			&mut random,
			&mut friendly,
			&mut enemies,
			1,
			1,
			EngagementType::SameCourse,
		)
		.unwrap();

		// friendly ship should have sp_list indicating CI (4 = torpedo CI)
		assert_eq!(hougeki.api_sp_list[0], 4, "torpedo CI sp_list should be 4");
		assert_eq!(hougeki.api_damage[0].len(), 2, "torpedo CI should deal 2 hits");
	}

	#[test]
	fn airbattle_mode_skips_shelling_and_torpedo() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

		let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let friend = sample_ship(&codex, bb_mst, 99);
		let enemy = sample_ship(&codex, dd_mst, 50);

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Sortie,
				battle_type: BattleType::AirBattle,
				is_sortie: true,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![friend],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
			},
		);

		// Airbattle should skip shelling and torpedo
		assert!(simulation.packet.hougeki1.is_none(), "airbattle should skip shelling");
		assert!(simulation.packet.hougeki2.is_none());
		assert!(simulation.packet.raigeki.is_none(), "airbattle should skip closing torpedo");
		assert!(
			simulation.packet.opening_attack.is_none(),
			"airbattle should skip opening torpedo"
		);
		assert_eq!(simulation.packet.hourai_flag, [0, 0, 0, 0]);
	}

	#[test]
	fn airbattle_mode_still_runs_kouku() {
		let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

		let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
		let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
		let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

		let mut carrier = sample_ship(&codex, cvl_mst, 50);
		carrier.slot_items = vec![slotitem_with_mst_id(bomber_id)];
		carrier.ship.api_onslot = [18, 0, 0, 0, 0];

		let enemy = sample_ship(&codex, dd_mst, 50);

		let simulation = simulate_day_battle_v1(
			&codex,
			BattleContext {
				mode: BattleMode::Sortie,
				battle_type: BattleType::AirBattle,
				is_sortie: true,
				friendly_formation_id: 1,
				enemy_formation_id: 1,
				engagement: EngagementType::SameCourse,
				friend_ships: vec![carrier],
				enemy_ships: vec![enemy],
				rng_seed: Some(1),
			},
		);

		// Kouku should still execute
		assert!(simulation.packet.kouku.is_some(), "airbattle should still run kouku");
		assert_eq!(simulation.packet.stage_flag, [1, 1, 1]);
	}

	#[test]
	fn sinking_protection_saves_non_taiha_ship_in_sortie() {
		let mut random = BattleRandom::new(Some(42));
		let mut ship = make_test_ship(30, 30, 30, 40);
		let dealt = ship.apply_damage(&mut random, 999, 1);
		assert!(ship.hp() >= 1, "ship must survive with sinking protection");
		assert!(dealt < 30, "dealt damage must be less than current HP");
	}

	#[test]
	fn flagship_always_survives_even_when_taiha() {
		let mut random = BattleRandom::new(Some(42));
		let mut ship = make_test_ship(5, 5, 5, 40);
		let dealt = ship.apply_damage(&mut random, 999, 0);
		assert!(ship.hp() >= 1, "flagship must always survive");
		assert!(dealt < 5);
	}

	#[test]
	fn taiha_advance_ship_can_be_sunk() {
		let mut random = BattleRandom::new(Some(42));
		let mut ship = make_test_ship(5, 5, 5, 40);
		let dealt = ship.apply_damage(&mut random, 999, 1);
		assert_eq!(ship.hp(), 0, "taiha-advance ship should be sunk");
		assert_eq!(dealt, 5);
	}

	#[test]
	fn practice_never_triggers_sinking_protection() {
		let mut random = BattleRandom::new(Some(42));
		let mut ship = make_test_ship_ctx(30, 30, 30, 40, true, false);
		let dealt = ship.apply_damage(&mut random, 999, 1);
		assert_eq!(ship.hp(), 0, "practice uses normal damage clamping");
		assert_eq!(dealt, 30);
	}

	#[test]
	fn enemy_ships_never_get_sinking_protection() {
		let mut random = BattleRandom::new(Some(42));
		let mut ship = make_test_ship_ctx(30, 30, 30, 40, false, true);
		let dealt = ship.apply_damage(&mut random, 999, 0);
		assert_eq!(ship.hp(), 0, "enemy ships should be sinkable");
		assert_eq!(dealt, 30);
	}

	#[test]
	fn win_rank_s_requires_no_friendly_sinking() {
		let friendly = vec![make_test_ship(40, 40, 30, 40)];
		let enemy = vec![make_test_ship(40, 40, 0, 40)];
		assert_eq!(calculate_win_rank(&friendly, &enemy), "S");
	}

	#[test]
	fn win_rank_downgraded_to_a_when_friendly_sunk() {
		let friendly = vec![
			make_test_ship(40, 40, 30, 40),
			make_test_ship(30, 30, 0, 30),
		];
		let enemy = vec![make_test_ship(40, 40, 0, 40)];
		assert_eq!(calculate_win_rank(&friendly, &enemy), "A");
	}

	#[test]
	fn win_rank_e_when_all_friendly_sunk() {
		let friendly = vec![make_test_ship(40, 40, 0, 40)];
		let enemy = vec![make_test_ship(40, 40, 20, 40)];
		assert_eq!(calculate_win_rank(&friendly, &enemy), "E");
	}

	#[test]
	fn win_rank_d_when_half_friendly_sunk() {
		let friendly = vec![
			make_test_ship(40, 40, 30, 40),
			make_test_ship(30, 30, 0, 30),
		];
		let enemy = vec![make_test_ship(40, 40, 35, 40)];
		assert_eq!(calculate_win_rank(&friendly, &enemy), "D");
	}

	fn make_test_ship(
		nowhp: i64,
		entry_hp: i64,
		current_hp: i64,
		maxhp: i64,
	) -> BattleRuntimeShip {
		make_test_ship_ctx(nowhp, entry_hp, current_hp, maxhp, true, true)
	}

	fn make_test_ship_ctx(
		nowhp: i64,
		entry_hp: i64,
		current_hp: i64,
		maxhp: i64,
		is_friendly: bool,
		is_sortie: bool,
	) -> BattleRuntimeShip {
		let mut ship = BattleRuntimeShip::new(
			BattleShipInput {
				ship: test_api_ship(nowhp, maxhp),
				slot_items: vec![],
				effect_list: vec![],
			},
			is_friendly,
			is_sortie,
		);
		ship.entry_hp = entry_hp;
		ship.current_hp = current_hp;
		ship
	}

	fn test_api_ship(nowhp: i64, maxhp: i64) -> KcApiShip {
		KcApiShip {
			api_id: 1,
			api_sortno: 1,
			api_ship_id: 1,
			api_lv: 1,
			api_exp: [0, 0, 0],
			api_nowhp: nowhp,
			api_maxhp: maxhp,
			api_soku: 10,
			api_leng: 1,
			api_slot: [-1; 5],
			api_onslot: [0; 5],
			api_slot_ex: 0,
			api_kyouka: [0; 7],
			api_backs: 1,
			api_fuel: 0,
			api_bull: 0,
			api_slotnum: 4,
			api_ndock_time: 0,
			api_ndock_item: [0; 2],
			api_srate: 0,
			api_cond: 49,
			api_karyoku: [0; 2],
			api_raisou: [0; 2],
			api_taiku: [0; 2],
			api_soukou: [0; 2],
			api_kaihi: [0; 2],
			api_taisen: [0; 2],
			api_sakuteki: [0; 2],
			api_lucky: [0; 2],
			api_locked: 0,
			api_locked_equip: 0,
			api_sally_area: 0,
			api_sp_effect_items: None,
		}
	}
}
