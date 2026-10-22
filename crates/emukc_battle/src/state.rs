use emukc_model::codex::Codex;

use crate::combined::{CombinedFleetRole, CombinedType};
use crate::outcome::{calculate_mvp, calculate_win_rank, verify_protected_ships_alive};
use crate::pt_imp::mark_anti_pt;
use crate::targeting::any_alive;
use crate::types::CombinedMembership;
use crate::types::{
    AirCorpsInput, BattleAirBaseAttack, BattleContext, BattleHougeki, BattleKouku,
    BattleOpeningAttack, BattleOutcome, BattlePacket, BattleRaigeki, BattleRuntimeShip,
    BattleSimulation, BattleType, NightBattlePacket, NightBattleSimulation, VanguardPlace,
};

/// Combined-fleet layout of [`BattleState::friendly`].
///
/// The two decks live in one contiguous vector — deck 1 at `..escort_start`,
/// deck 2 at `escort_start..` — so that a phase in which the enemy fires at the
/// whole friendly force is just the whole slice, and a phase belonging to one
/// deck is a sub-slice. `escort_start` is deck 1's actual ship count, which is
/// **not** the packet's escort offset: the client puts deck 2 at index 6 no
/// matter how few ships deck 1 holds. The phases work entirely in this space;
/// [`finalize_day`](BattleState::finalize_day) translates the finished packet
/// into the client's through [`combined_packet`](crate::combined_packet).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CombinedLayout {
    pub(crate) combined_type: CombinedType,
    pub(crate) escort_start: usize,
}

/// 阻塞気球 and 深海観測阻塞気球.
const BALLOONS: [i64; 2] = [513, 1640];

/// Mark the first ship of a friendly fleet, or of one deck of it, as its flagship.
fn mark_flagship(fleet: &mut [BattleRuntimeShip]) {
    if let Some(flagship) = fleet.first_mut() {
        flagship.is_flagship = true;
    }
}

/// Give each ship of a fleet in 警戒陣 (formation 6) her place in the line.
fn place_in_vanguard(fleet: &mut [BattleRuntimeShip], formation_id: i64) {
    if formation_id != 6 {
        return;
    }
    let fleet_len = fleet.len();
    for (index, ship) in fleet.iter_mut().enumerate() {
        ship.vanguard = Some(VanguardPlace {
            number: index + 1,
            fleet_len,
        });
    }
}

/// All mutable state for a single battle simulation.
///
/// Created from a [`BattleContext`], mutated by phase functions, then
/// consumed by [`finalize_day`](Self::finalize_day) or
/// [`finalize_night`](Self::finalize_night) to produce the simulation output.
///
/// `friendly` and `enemy` are `pub(crate)` because every phase function needs
/// `&mut` access to them. All other fields are private with setters.
pub(crate) struct BattleState {
    pub(crate) friendly: Vec<BattleRuntimeShip>,
    pub(crate) enemy: Vec<BattleRuntimeShip>,

    battle_type: BattleType,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: super::types::EngagementType,

    /// The air corps still to attack, and what they have left.
    pub(crate) air_corps: Vec<AirCorpsInput>,
    pub(crate) air_base_attack: Vec<BattleAirBaseAttack>,
    kouku: Option<BattleKouku>,
    opening_attack: Option<BattleOpeningAttack>,
    opening_taisen: Option<BattleHougeki>,
    hougeki1: Option<BattleHougeki>,
    hougeki2: Option<BattleHougeki>,
    hougeki3: Option<BattleHougeki>,
    raigeki: Option<BattleRaigeki>,

    /// `None` for an ordinary single-fleet battle.
    combined: Option<CombinedLayout>,
    /// Where the escort fleet begins in `enemy`, laid out the way
    /// [`CombinedLayout`] lays out `friendly`; `None` for an enemy single fleet.
    enemy_escort_start: Option<usize>,

    stage_flag: [i64; 3],
    hourai_flag: [i64; 4],
    opening_taisen_flag: i64,
    has_bb_class_at_start: bool,
    /// How thick a smoke screen the player's fleet laid, 0 for none (`api_smoke_type`).
    smoke_type: i64,
    /// Whether the cell raises barrage balloons (`api_balloon_cell`).
    balloon_cell: bool,
    /// Whether each side, friendly then enemy, has made its special attack.
    pub(crate) special_attack_used: [bool; 2],
    /// Each deck's fleet LOS for artillery spotting, friendly then enemy, main then escort, once
    /// taken (`simulation::spotting_los`).
    pub(crate) fleet_los: [[Option<i64>; 2]; 2],
}

impl BattleState {
    /// Build initial state from a battle context.
    pub fn from_context(codex: &Codex, context: BattleContext) -> Self {
        let is_sortie = context.is_sortie;
        let mut friendly = context
            .friend_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, true, is_sortie))
            .collect::<Vec<_>>();
        place_in_vanguard(&mut friendly, context.friendly_formation_id);
        mark_flagship(&mut friendly);
        mark_anti_pt(codex, &mut friendly);

        // A combined fleet appends deck 2 to the same vector and tags both
        // decks. With no combined setup this loop does not run, `friendly` is
        // byte-for-byte what it was before, and every ship keeps
        // `combined_role: None` — which is what keeps the single-fleet RNG
        // stream identical.
        let combined = context.combined.map(|setup| {
            let combined_type = setup.combined_type;
            for ship in &mut friendly {
                ship.combined = Some(CombinedMembership {
                    combined_type,
                    role: CombinedFleetRole::Main,
                });
            }
            let escort_start = friendly.len();
            friendly.extend(setup.escort_ships.into_iter().map(|s| {
                BattleRuntimeShip::new(s, true, is_sortie)
                    .in_combined_fleet(combined_type, CombinedFleetRole::Escort)
            }));
            mark_flagship(&mut friendly[escort_start..]);
            mark_anti_pt(codex, &mut friendly[escort_start..]);
            CombinedLayout {
                combined_type,
                escort_start,
            }
        });
        let mut enemy = context
            .enemy_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, false, is_sortie))
            .collect::<Vec<_>>();
        place_in_vanguard(&mut enemy, context.enemy_formation_id);
        // Same shape as the friendly side above, and the same guarantee: with
        // no escort fleet nothing is tagged and the vector is what it was.
        let enemy_escort_start = (!context.enemy_escort_ships.is_empty()).then(|| {
            for ship in &mut enemy {
                ship.enemy_deck = Some(CombinedFleetRole::Main);
            }
            let escort_start = enemy.len();
            enemy.extend(context.enemy_escort_ships.into_iter().map(|s| {
                BattleRuntimeShip::new(s, false, is_sortie).in_enemy_deck(CombinedFleetRole::Escort)
            }));
            escort_start
        });

        Self {
            friendly,
            enemy,
            battle_type: context.battle_type,
            friendly_formation_id: context.friendly_formation_id,
            enemy_formation_id: context.enemy_formation_id,
            engagement: context.engagement,
            air_corps: context.air_corps,
            air_base_attack: Vec::new(),
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
            combined,
            enemy_escort_start,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
            smoke_type: 0,
            balloon_cell: false,
            special_attack_used: [context.special_attacks_made.gunnery, false],
            fleet_los: [[None; 2]; 2],
        }
    }

    /// Build minimal state for night battle finalization.
    /// Avoids the full `BattleContext` → runtime-ship pipeline when the ships
    /// have already been mutated by day battle phases.
    pub fn for_night(
        friendly: Vec<BattleRuntimeShip>,
        enemy: Vec<BattleRuntimeShip>,
        friendly_formation_id: i64,
        enemy_formation_id: i64,
        engagement: super::types::EngagementType,
    ) -> Self {
        Self {
            friendly,
            enemy,
            battle_type: BattleType::Normal,
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_corps: Vec::new(),
            air_base_attack: Vec::new(),
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
            combined: None,
            enemy_escort_start: None,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
            smoke_type: 0,
            balloon_cell: false,
            special_attack_used: [false; 2],
            fleet_los: [[None; 2]; 2],
        }
    }

    // -- Read accessors (for phase dispatch) --

    pub(crate) fn battle_type(&self) -> BattleType {
        self.battle_type
    }

    pub(crate) fn friendly_formation_id(&self) -> i64 {
        self.friendly_formation_id
    }

    pub(crate) fn enemy_formation_id(&self) -> i64 {
        self.enemy_formation_id
    }

    pub(crate) fn engagement(&self) -> super::types::EngagementType {
        self.engagement
    }

    /// The combined-fleet layout, or `None` in an ordinary single-fleet battle.
    pub(crate) fn combined(&self) -> Option<CombinedLayout> {
        self.combined
    }

    /// Where the enemy escort fleet begins in `enemy`, or `None` when the enemy
    /// is a single fleet.
    pub(crate) fn enemy_escort_start(&self) -> Option<usize> {
        self.enemy_escort_start
    }

    // -- Setters (for phase functions to write outputs) --

    pub(crate) fn set_kouku(&mut self, kouku: BattleKouku) {
        self.kouku = Some(kouku);
    }

    pub(crate) fn set_opening_attack(&mut self, attack: Option<BattleOpeningAttack>) {
        self.opening_attack = attack;
    }

    pub(crate) fn set_opening_taisen(&mut self, taisen: Option<BattleHougeki>) {
        self.opening_taisen = taisen;
    }

    pub(crate) fn set_opening_taisen_flag(&mut self, flag: bool) {
        self.opening_taisen_flag = i64::from(flag);
    }

    pub(crate) fn set_hougeki1(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki1 = hougeki;
    }

    pub(crate) fn set_hougeki2(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki2 = hougeki;
    }

    /// Only a combined battle fills the third shelling round: `battle` puts
    /// deck 1's second round here, `battle_water` puts deck 2's single round
    /// here. A single-fleet battle leaves it `None`.
    pub(crate) fn set_hougeki3(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki3 = hougeki;
    }

    pub(crate) fn set_raigeki(&mut self, raigeki: Option<BattleRaigeki>) {
        self.raigeki = raigeki;
    }

    pub(crate) fn set_stage_flag(&mut self, flags: [i64; 3]) {
        self.stage_flag = flags;
    }

    pub(crate) fn set_hourai_flag(&mut self, index: usize, value: i64) {
        debug_assert!(index < 4, "hourai_flag index out of bounds: {index}");
        self.hourai_flag[index] = value;
    }

    pub(crate) fn set_has_bb_class_at_start(&mut self, value: bool) {
        self.has_bb_class_at_start = value;
    }

    /// Raise each side's barrage balloons: every ship learns how many ships of her fleet, both
    /// decks of a combined one, fly one (阻塞気球 513 or 深海観測阻塞気球 1640), at most 3
    /// (`Fleet.getNumBalloons`).
    // ponytail: counted once, as the battle opens; the source counts the ships afloat at each
    // attack, so a balloon ship sunk mid-battle still counts here. No regular map's enemy flies one.
    pub(crate) fn raise_balloons(&mut self) {
        self.balloon_cell = true;
        for fleet in [&mut self.friendly, &mut self.enemy] {
            let flying = fleet
                .iter()
                .filter(|ship| {
                    ship.is_alive()
                        && ship
                            .slot_items
                            .iter()
                            .any(|item| BALLOONS.contains(&item.api_slotitem_id))
                })
                .count()
                .min(3) as i64;
            for ship in fleet.iter_mut() {
                ship.balloons = flying;
            }
        }
    }

    /// Put every friendly ship in a smoke screen of `level`.
    pub(crate) fn lay_smoke(&mut self, level: i64) {
        self.smoke_type = level;
        for ship in &mut self.friendly {
            ship.smoke = level;
        }
    }

    pub(crate) fn has_bb_class_at_start(&self) -> bool {
        self.has_bb_class_at_start
    }

    pub(crate) fn kouku(&self) -> Option<&BattleKouku> {
        self.kouku.as_ref()
    }

    // -- Finalizers --

    /// Consume state, verify invariants, produce the day battle simulation result.
    pub fn finalize_day(self) -> BattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let can_midnight = matches!(self.battle_type, BattleType::Normal | BattleType::AirBattle)
            && any_alive(&self.friendly)
            && any_alive(&self.enemy);

        let mut packet = BattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps: self.friendly.iter().map(|ship| ship.hp().max(0)).collect(),
            enemy_nowhps: self.enemy.iter().map(|ship| ship.hp().max(0)).collect(),
            smoke_type: self.smoke_type,
            balloon_cell: i64::from(self.balloon_cell),
            atoll_cell: 0,
            midnight_flag: i64::from(can_midnight),
            search: [1, 1],
            stage_flag: self.stage_flag,
            air_base_attack: self.air_base_attack,
            kouku: self.kouku,
            opening_taisen_flag: self.opening_taisen_flag,
            opening_taisen: self.opening_taisen,
            opening_flag: i64::from(self.opening_attack.is_some()),
            opening_attack: self.opening_attack,
            hourai_flag: self.hourai_flag,
            hougeki1: self.hougeki1,
            hougeki2: self.hougeki2,
            hougeki3: self.hougeki3,
            raigeki: self.raigeki,
        };

        // The phases wrote every friendly index in this module's contiguous
        // space; the client reads 第2艦隊 from index 6. Translating here is the
        // one chokepoint every caller passes through.
        if let Some(layout) = self.combined {
            crate::combined_packet::remap_day_packet(
                &mut packet,
                layout.escort_start,
                self.enemy.len(),
            );
        }

        if let Some(escort_start) = self.enemy_escort_start {
            crate::combined_packet::remap_day_packet_enemy(
                &mut packet,
                escort_start,
                self.friendly.len(),
            );
        }

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight,
        };

        BattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }

    /// Consume state, verify invariants, produce the night battle simulation result.
    #[expect(clippy::too_many_arguments)]
    pub fn finalize_night(
        self,
        friendly_nowhps: Vec<i64>,
        friendly_maxhps: Vec<i64>,
        enemy_nowhps: Vec<i64>,
        enemy_maxhps: Vec<i64>,
        flare_pos: [i64; 2],
        touch_plane: [i64; 2],
        hougeki: Option<crate::types::BattleNightHougeki>,
    ) -> NightBattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight: false,
        };

        let mut packet = NightBattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps,
            friendly_maxhps,
            enemy_nowhps,
            enemy_maxhps,
            touch_plane,
            flare_pos,
            hougeki,
        };

        // A combined night battle is fought by 第2艦隊 alone, so the friendly
        // vector holds nothing but the escort deck — and the client still
        // expects it at indices 6..=11. The ships carry their own deck tag, so
        // no caller has to declare it.
        if self.friendly.first().is_some_and(BattleRuntimeShip::is_escort_deck) {
            crate::combined_packet::remap_night_packet(&mut packet);
        }
        // Likewise for the enemy: against a combined fleet only one of its
        // decks fights at night, and the escort fleet is read from 6..=11.
        if self.enemy.first().is_some_and(BattleRuntimeShip::is_escort_deck) {
            crate::combined_packet::remap_night_packet_enemy(&mut packet);
        }

        NightBattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use emukc_model::{codex::Codex, kc2::types::KcShipType};

    #[test]
    fn a_fleet_in_vanguard_splits_into_halves_rounded_down() {
        let front = |fleet_len| {
            (1..=fleet_len)
                .map(|number| {
                    VanguardPlace {
                        number,
                        fleet_len,
                    }
                    .front()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(front(5), [true, true, false, false, false]);
        assert_eq!(front(6), [true, true, true, false, false, false]);
        assert_eq!(front(7), [true, true, true, false, false, false, false]);
    }

    #[test]
    fn a_balloon_cell_counts_each_sides_balloons_up_to_three() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ship = |balloon: bool| {
            let mut ship = sample_ship(&codex, dd, 1);
            if balloon {
                ship.slot_items = vec![crate::test_utils::slotitem_with_mst_id(513)];
            }
            ship
        };
        let mut context = BattleContext::head_on(
            BattleType::Normal,
            true,
            vec![ship(true), ship(true), ship(true), ship(true), ship(false)],
            vec![ship(false), ship(false)],
        );
        context.friend_ships[3].ship.api_nowhp = 0;
        let mut state = BattleState::from_context(&codex, context);
        state.raise_balloons();
        // Four carry one, three afloat.
        assert!(state.friendly.iter().all(|ship| ship.balloons == 3));
        assert!(state.enemy.iter().all(|ship| ship.balloons == 0));
        assert_eq!(state.finalize_day().packet.balloon_cell, 1);
    }

    #[test]
    fn a_battle_places_the_ships_of_a_fleet_in_vanguard() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let fleet = |count| (0..count).map(|_| sample_ship(&codex, dd, 1)).collect::<Vec<_>>();
        let mut context = BattleContext::head_on(BattleType::Normal, true, fleet(6), fleet(5));
        context.friendly_formation_id = 6;
        let state = BattleState::from_context(&codex, context);

        let fronts = state.friendly.iter().map(BattleRuntimeShip::in_vanguard_front);
        assert_eq!(fronts.collect::<Vec<_>>(), [true, true, true, false, false, false]);
        assert!(state.enemy.iter().all(|ship| ship.vanguard.is_none()), "the enemy is in line");
    }
}
