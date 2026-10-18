//! Runtime battle types — ship state, battle context, and simulation output.
//! These types carry mutable battle state and top-level simulation results.

use emukc_model::kc2::{KcApiShip, KcApiSlotItem, KcSortieResultRank};

use super::domain::{AirState, BattleType, EngagementType};
use super::packet::{
    BattleAirBaseAttack, BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack,
    BattleRaigeki, DamageCell,
};
use crate::combined::{CombinedFleetRole, CombinedType};
use crate::random::BattleRng;

#[derive(Debug, Clone)]
pub struct BattleShipInput {
    pub ship: KcApiShip,
    pub slot_items: Vec<KcApiSlotItem>,
    /// The stored proficiency experience of each of `slot_items`, by position. Empty when
    /// unknown (an enemy, a rival, a test): the level's representative experience stands in.
    pub aircraft_exp: Vec<i64>,
    pub effect_list: Vec<i64>,
    pub married: bool,
}

#[derive(Debug, Clone)]
pub struct BattleRuntimeShip {
    pub ship: KcApiShip,
    pub slot_items: Vec<KcApiSlotItem>,
    /// See [`BattleShipInput::aircraft_exp`].
    pub aircraft_exp: Vec<i64>,
    pub effect_list: Vec<i64>,
    /// Current HP -- only mutable through [`take_hit`](Self::take_hit).
    pub(crate) current_hp: i64,
    /// HP at the start of this battle node (before any combat phases).
    /// Used to determine sinking protection eligibility.
    pub entry_hp: i64,
    pub damage_dealt: i64,
    /// Whether this ship belongs to the player (friendly) side.
    pub(crate) is_friendly: bool,
    /// Whether this battle is a sortie (true) or practice (false).
    /// Sinking protection only applies during sorties.
    pub(crate) is_sortie: bool,
    /// Whether she leads a friendly fleet: the first ship of a single fleet or of either deck of a
    /// combined fleet, which a sortie never loses (`kcships.js` `69097abc` `:45`, `:1825`).
    pub(crate) is_flagship: bool,
    /// This ship's place in a combined fleet, or `None` in an ordinary
    /// single-fleet battle. Phase functions read the role to decide who
    /// participates (deck 1 never opens with ASW or torpedoes, and a shelling
    /// round belongs to exactly one deck); `damage` reads both halves to look up
    /// the 連合艦隊補正.
    pub(crate) combined: Option<CombinedMembership>,
    /// This ship's deck in an enemy combined fleet, or `None` for a friendly
    /// ship and for an enemy single fleet. Kept apart from `combined` because
    /// an enemy fleet has no [`CombinedType`] to go with the deck.
    pub(crate) enemy_deck: Option<CombinedFleetRole>,
    /// Her place in the line when her fleet is in 警戒陣, `None` in any other
    /// formation.
    pub(crate) vanguard: Option<VanguardPlace>,
    pub married: bool,
}

/// A ship's place in a fleet in 警戒陣 (`Ship.num`, `fleet.ships.length`).
/// A ship sunk or sent home keeps her number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VanguardPlace {
    /// 1-based.
    pub(crate) number: usize,
    pub(crate) fleet_len: usize,
}

impl VanguardPlace {
    /// Whether she is in the front half (`VANGUARD1`): the first half of the
    /// fleet, rounded down (`getFormation`).
    pub(crate) fn front(self) -> bool {
        self.number <= self.fleet_len / 2
    }
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
            aircraft_exp: input.aircraft_exp,
            effect_list: input.effect_list,
            is_friendly,
            is_sortie,
            is_flagship: false,
            combined: None,
            enemy_deck: None,
            vanguard: None,
            married: input.married,
        }
    }

    /// Tag this ship with its place in a combined fleet.
    ///
    /// Only [`BattleState::from_context`](crate::state::BattleState::from_context)
    /// calls this; every other construction path leaves `combined` as `None`,
    /// which is what makes the single-fleet code paths behave exactly as before.
    pub(crate) fn in_combined_fleet(
        mut self,
        combined_type: CombinedType,
        role: CombinedFleetRole,
    ) -> Self {
        self.combined = Some(CombinedMembership {
            combined_type,
            role,
        });
        self
    }

    /// Tag this ship with its deck in an enemy combined fleet.
    pub(crate) fn in_enemy_deck(mut self, role: CombinedFleetRole) -> Self {
        self.enemy_deck = Some(role);
        self
    }

    /// Place this ship at `number` of a fleet of `fleet_len` in 警戒陣.
    #[cfg(test)]
    pub(crate) fn in_vanguard(mut self, number: usize, fleet_len: usize) -> Self {
        self.vanguard = Some(VanguardPlace {
            number,
            fleet_len,
        });
        self
    }

    /// Whether she is in the front half of a fleet in 警戒陣.
    pub(crate) fn in_vanguard_front(&self) -> bool {
        self.vanguard.is_some_and(VanguardPlace::front)
    }

    /// This ship's deck — on either side — or `None` in a single fleet.
    pub(crate) fn combined_role(&self) -> Option<CombinedFleetRole> {
        self.combined.map(|m| m.role).or(self.enemy_deck)
    }

    /// True when this ship is in the escort deck (第2艦隊) of a combined fleet.
    ///
    /// Public because the sortie session stores both decks in one vector and
    /// needs to find the boundary again — a night battle is fought by 第2艦隊
    /// alone. The tag travels with the ship, so no caller has to track it.
    pub fn is_escort_deck(&self) -> bool {
        self.combined_role() == Some(CombinedFleetRole::Escort)
    }

    /// True when this ship is in the main deck (第1艦隊) of a combined fleet.
    /// False for a single-fleet ship — it has no deck at all.
    pub fn is_main_deck(&self) -> bool {
        self.combined_role() == Some(CombinedFleetRole::Main)
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

    /// Override current HP for debug transforms. Only used by
    /// [`debug_overlay`](crate::debug_overlay) after applying event transforms.
    pub(crate) fn set_hp_for_debug(&mut self, hp: i64) {
        self.current_hp = hp.max(0);
    }

    /// Land a hit of `raw` damage on this ship: sinking protection, the HP lost,
    /// and the cell the battle log shows for it. Every attack that damages a ship
    /// goes through here; what the attacker is credited with is up to the caller.
    ///
    /// An enemy shows the raw damage, so the client plays the overkill; a friendly
    /// ship shows what she lost, so the client's HP bar stays true. `shield` marks
    /// a hit an escort took for the flagship (かばう).
    pub(crate) fn take_hit(&mut self, rng: &mut impl BattleRng, raw: i64, shield: bool) -> Hit {
        let (raw, dealt) = self.apply_damage(rng, raw);
        let shown = if self.is_friendly {
            dealt
        } else {
            raw
        };
        Hit {
            cell: if shield {
                DamageCell::Shielded(shown)
            } else {
                DamageCell::Plain(shown)
            },
            dealt,
        }
    }

    /// Whether a lethal hit is turned into proportional damage instead: a
    /// friendly ship in a sortie that is her fleet's flagship, or was not in
    /// taiha (HP <= 25% max) when the battle node began.
    pub(crate) fn protected_from_sinking(&self) -> bool {
        let was_taiha_at_entry = self.entry_hp * 4 <= self.ship.api_maxhp;
        self.is_friendly && self.is_sortie && (self.is_flagship || !was_taiha_at_entry)
    }

    /// Apply damage with sinking protection (轟沈ストッパー).
    ///
    /// In real `KanColle`:
    /// - Friendly ships that were **not** in taiha (HP <= 25% max) at the start of
    ///   the battle node cannot be sunk. Lethal damage is replaced with
    ///   proportional damage: `floor(0.5 * H + 0.3 * rand(0..H))`, `H` being
    ///   the hit points the ship has when it is hit. That is always less than
    ///   `H`, and nothing at 1.
    /// - A flagship (`is_flagship`) can **never** be sunk regardless of HP state.
    /// - Protection only applies to friendly ships during sorties (not practice).
    ///
    /// Returns `(raw_damage, effective_damage)` where raw is the input damage
    /// and effective is the HP actually subtracted (after clamping/protection).
    pub(super) fn apply_damage(&mut self, rng: &mut impl BattleRng, raw_damage: i64) -> (i64, i64) {
        if self.is_sunk() {
            return (0, 0);
        }

        // Enemy ships in a sortie take uncapped damage so the client renders
        // overkill (HP goes negative) and the attacker's damage_dealt credits the
        // full hit toward MVP. Friendly ships and all practice damage stay clamped
        // to current HP — friendly lethal hits route through sinking protection
        // below. Downstream HP consumers (win rank, nowhps wire, transcript)
        // already clamp with `.max(0)`.
        let effective = if !self.is_friendly && self.is_sortie {
            raw_damage
        } else {
            raw_damage.min(self.current_hp)
        };

        if effective >= self.current_hp && self.protected_from_sinking() {
            // Replace lethal damage with proportional damage (割合ダメージ),
            // taken from what the ship has left now.
            let h = self.current_hp;
            let dealt = if h > 1 {
                (5 * h + 3 * rng.roll_range(0, h)) / 10
            } else {
                0
            };
            self.current_hp -= dealt;
            return (raw_damage, dealt);
        }

        self.current_hp -= effective;
        (raw_damage, effective)
    }
}

/// What a hit left on the ship it struck: see [`BattleRuntimeShip::take_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hit {
    /// What the battle log shows for the hit.
    pub(crate) cell: DamageCell,
    /// The hit points the ship lost.
    pub(crate) dealt: i64,
}

#[cfg(test)]
impl From<BattleShipInput> for BattleRuntimeShip {
    /// Convenience conversion for tests.
    /// Defaults to friendly sortie ship (sinking protection enabled).
    /// Use `BattleRuntimeShip::new(input, is_enemy, is_sortie)` for specific contexts.
    fn from(input: BattleShipInput) -> Self {
        Self::new(input, false, true)
    }
}

/// A ship's place in a combined fleet: which flavour of fleet, and which of its
/// two decks. The two always travel together — a deck without a fleet type
/// cannot be looked up in the correction table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombinedMembership {
    pub(crate) combined_type: CombinedType,
    pub(crate) role: CombinedFleetRole,
}

/// The escort deck of a friendly combined fleet, plus which combined fleet it
/// is. The two always travel together — an escort deck with no type, or a type
/// with no escort deck, is not a state the game can be in.
#[derive(Debug, Clone)]
pub struct CombinedSetup {
    pub combined_type: CombinedType,
    /// 第2艦隊. Occupies packet indices 6..=11 regardless of how many ships
    /// `friend_ships` holds.
    pub escort_ships: Vec<BattleShipInput>,
}

/// The fleet special attacks a fleet has made since its sortie began.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialAttacksMade {
    /// A flagship's special gunnery attack (100 to 103, 105, 106): one a sortie.
    pub gunnery: bool,
    /// The Kongou class's night assaults (104), counted apart: three a sortie.
    pub kongou_assaults: u8,
}

impl SpecialAttacksMade {
    /// How many night assaults a sortie allows (wikiwiki 金剛改二丙; the source's
    /// `numSpecialKongou >= 3`).
    pub(crate) const KONGOU_ASSAULTS_A_SORTIE: u8 = 3;

    /// Take in the special attacks the friendly fleet made in a battle
    /// ([`BattlePacket::special_attacks`], [`NightBattlePacket::special_attacks`]).
    pub fn record(&mut self, special_attacks: &[i64]) {
        for attack_type in special_attacks {
            match attack_type {
                104 => self.kongou_assaults += 1,
                _ => self.gunnery = true,
            }
        }
    }
}

/// The friendly rows of a shelling phase that are fleet special attacks (100 to 106).
fn special_attacks_in(eflags: &[i64], attack_types: &[i64]) -> impl Iterator<Item = i64> {
    eflags
        .iter()
        .zip(attack_types)
        .filter(|(eflag, attack_type)| **eflag == 0 && (100..=106).contains(*attack_type))
        .map(|(_, attack_type)| *attack_type)
}

impl BattlePacket {
    /// The fleet special attacks the friendly fleet made, by their `api_at_type`.
    pub fn special_attacks(&self) -> Vec<i64> {
        [&self.hougeki1, &self.hougeki2, &self.hougeki3]
            .into_iter()
            .flatten()
            .flat_map(|round| special_attacks_in(&round.api_at_eflag, &round.api_at_type))
            .collect()
    }
}

impl NightBattlePacket {
    /// The fleet special attacks the friendly fleet made, by their `api_sp_list`.
    pub fn special_attacks(&self) -> Vec<i64> {
        self.hougeki
            .iter()
            .flat_map(|hougeki| special_attacks_in(&hougeki.api_at_eflag, &hougeki.api_sp_list))
            .collect()
    }
}

/// Input parameters for a day battle simulation.
#[derive(Debug, Clone)]
pub struct BattleContext {
    pub battle_type: BattleType,
    /// Whether this is a sortie battle (true) or practice (false).
    /// Sinking protection only applies during sorties.
    pub is_sortie: bool,
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: EngagementType,
    /// 第1艦隊 when `combined` is set, otherwise the whole fleet.
    pub friend_ships: Vec<BattleShipInput>,
    /// The enemy main fleet when `enemy_escort_ships` is non-empty, otherwise
    /// the whole enemy fleet.
    pub enemy_ships: Vec<BattleShipInput>,
    /// The escort fleet of an enemy combined fleet; empty for a single fleet.
    pub enemy_escort_ships: Vec<BattleShipInput>,
    /// `None` for an ordinary single-fleet battle.
    pub combined: Option<CombinedSetup>,
    /// The air corps sent against this battle's cell; empty for most battles,
    /// and then the battle draws not one random number more.
    pub air_corps: Vec<AirCorpsInput>,
    /// What the fleet has made of its special attacks on this sortie.
    pub special_attacks_made: SpecialAttacksMade,
}

/// An air corps attacking the enemy fleet before the fleets meet.
#[derive(Debug, Clone)]
pub struct AirCorpsInput {
    /// The air corps, by its id within the area.
    pub base_rid: i64,
    /// How many times it attacks this cell: once for each time it was pointed here.
    pub waves: usize,
    /// Its squadrons that fly, in squadron order.
    pub squadrons: Vec<AirSquadronInput>,
}

/// One squadron of an attacking air corps.
#[derive(Debug, Clone)]
pub struct AirSquadronInput {
    /// Squadron slot, 1-based.
    pub squadron_id: i64,
    /// Equipment master id.
    pub mst_id: i64,
    /// Aircraft left.
    pub count: i64,
    /// Proficiency level, 0 to 7.
    pub alv: i64,
    /// Stored proficiency experience; `None` reads the level's representative value.
    pub exp: Option<i64>,
}

impl BattleContext {
    /// Build a context for the fixed head-on setup every battle test uses:
    /// formation 1 on both sides, `EngagementType::SameCourse`.
    pub fn head_on(
        battle_type: BattleType,
        is_sortie: bool,
        friend_ships: Vec<BattleShipInput>,
        enemy_ships: Vec<BattleShipInput>,
    ) -> Self {
        Self {
            battle_type,
            is_sortie,
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships,
            enemy_ships,
            enemy_escort_ships: Vec::new(),
            combined: None,
            air_corps: Vec::new(),
            special_attacks_made: SpecialAttacksMade::default(),
        }
    }
}

/// Input parameters for [`execute_night`](crate::execute_night).
pub struct NightBattleInput {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: EngagementType,
    pub air_state: Option<AirState>,
    /// Whether this is a night-start (開幕夜戦) battle and not the night after a day battle.
    pub night_start: bool,
    /// What the fleet has made of its special attacks on this sortie.
    pub special_attacks_made: SpecialAttacksMade,
}

#[derive(Debug, Clone)]
pub struct BattlePacket {
    pub formation: [i64; 3],
    pub friendly_nowhps: Vec<i64>,
    pub enemy_nowhps: Vec<i64>,
    pub smoke_type: i64,
    pub balloon_cell: i64,
    pub atoll_cell: i64,
    pub midnight_flag: i64,
    pub search: [i64; 2],
    pub stage_flag: [i64; 3],
    /// Air corps attacks, in the order they were flown; empty when none was sent.
    pub air_base_attack: Vec<BattleAirBaseAttack>,
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

/// Battle result: win rank, MVP ship index, and midnight eligibility.
#[derive(Debug, Clone)]
pub struct BattleOutcome {
    pub win_rank: KcSortieResultRank,
    pub mvp: i64,
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
