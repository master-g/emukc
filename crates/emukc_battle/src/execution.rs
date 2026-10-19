//! Complete battle execution entry points.
//!
//! This module owns the required ordering between raw simulation and the
//! post-simulation debug overlay.

use emukc_model::codex::Codex;

use crate::debug_overlay::{apply_day_debug, apply_night_debug};
use crate::random::BattleRng;
use crate::simulation::{simulate_day, simulate_night};
use crate::state::BattleState;
use crate::types::{
    BattleContext, BattleRuntimeShip, BattleSimulation, NightBattleInput, NightBattleSimulation,
};

/// Execute a day battle and apply the debug policy from `Codex`.
///
/// The raw simulation consumes `rng` sequentially across its phases. The
/// post-simulation debug overlay consumes no RNG, preserving that stream.
pub fn execute_day(
    codex: &Codex,
    context: BattleContext,
    rng: &mut impl BattleRng,
) -> BattleSimulation {
    apply_day_debug(
        simulate_day(codex, context, rng),
        codex.game_cfg.god_mode,
        codex.game_cfg.one_hit_kill,
    )
}

/// Execute a night battle and apply the debug policy from `Codex`.
///
/// The post-simulation debug overlay consumes no RNG, so this entry point
/// preserves the raw night simulation's RNG stream.
pub fn execute_night(
    codex: &Codex,
    input: NightBattleInput,
    rng: &mut impl BattleRng,
) -> NightBattleSimulation {
    apply_night_debug(
        simulate_night(codex, input, rng),
        codex.game_cfg.god_mode,
        codex.game_cfg.one_hit_kill,
    )
}

/// A night-start (開幕夜戦) battle: whoever fought, plus 第1艦隊 when it did not.
pub struct SpMidnightSimulation {
    /// 第1艦隊, which sits a combined night battle out; empty for a single fleet.
    pub main_deck: Vec<BattleRuntimeShip>,
    /// The night simulation, fought by 第2艦隊 alone when the fleet is combined.
    pub night: NightBattleSimulation,
}

/// Execute a night-start battle from a day [`BattleContext`].
///
/// A night-start cell has no day phase, but the fleets still have to be built
/// the same way — a combined fleet only behaves like one because
/// [`BattleState::from_context`] tags both decks, and the night packet's index
/// remap keys off those tags. So the context goes through the same builder and
/// only then splits: 第2艦隊 fights, 第1艦隊 is handed back untouched for the
/// caller to report.
pub fn execute_sp_midnight(
    codex: &Codex,
    context: BattleContext,
    rng: &mut impl BattleRng,
) -> SpMidnightSimulation {
    let friendly_formation_id = context.friendly_formation_id;
    let enemy_formation_id = context.enemy_formation_id;
    let engagement = context.engagement;
    let special_attacks_made = context.special_attacks_made;

    let state = BattleState::from_context(codex, context);
    let escort_start = state.combined().map_or(0, |layout| layout.escort_start);
    let mut friendly = state.friendly;
    let enemy = state.enemy;
    let main_deck: Vec<BattleRuntimeShip> = friendly.drain(..escort_start).collect();

    let night = execute_night(
        codex,
        NightBattleInput {
            special_attacks_made,
            friendly,
            enemy,
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_state: None,
            night_start: true,
        },
        rng,
    );

    SpMidnightSimulation {
        main_deck,
        night,
    }
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::{KcShipType, KcSlotItemType3};

    use super::{execute_day, execute_night, execute_sp_midnight};
    use crate::combined::CombinedType;
    use crate::debug_overlay::{apply_day_debug, apply_night_debug};
    use crate::random::{BattleRng, SeededRng};
    use crate::simulation::{simulate_day, simulate_night};
    use crate::test_utils::{
        first_ship_mst_by_type, first_slotitem_mst_by_type, sample_ship, slotitem_with_mst_id,
    };
    use crate::types::{
        BattleContext, BattleRuntimeShip, BattleType, CombinedSetup, EngagementType,
        NightBattleInput,
    };

    const SEED: u64 = 0xE7EC_0710;

    fn load_codex(god_mode: bool, one_hit_kill: bool) -> Codex {
        let mut codex = Codex::load_without_cache_source("../../.data/codex")
            .expect("load codex from ../../.data/codex (run `cargo run -- bootstrap` first)");
        codex.game_cfg.god_mode = god_mode;
        codex.game_cfg.one_hit_kill = one_hit_kill;
        codex
    }

    fn day_context(codex: &Codex) -> BattleContext {
        BattleContext {
            special_attacks_made: Default::default(),
            use_smoke: false,
            balloon_cell: false,
            battle_type: BattleType::Normal,
            is_sortie: true,
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: vec![sample_ship(codex, 79, 99), sample_ship(codex, 79, 99)],
            enemy_ships: vec![sample_ship(codex, 412, 99), sample_ship(codex, 412, 99)],
            enemy_escort_ships: Vec::new(),
            air_corps: Vec::new(),
            combined: None,
        }
    }

    fn night_input(codex: &Codex) -> NightBattleInput {
        NightBattleInput {
            special_attacks_made: Default::default(),
            friendly: vec![
                BattleRuntimeShip::new(sample_ship(codex, 79, 99), true, true),
                BattleRuntimeShip::new(sample_ship(codex, 79, 99), true, true),
            ],
            enemy: vec![
                BattleRuntimeShip::new(sample_ship(codex, 412, 99), false, true),
                BattleRuntimeShip::new(sample_ship(codex, 412, 99), false, true),
            ],
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            air_state: None,
            night_start: false,
        }
    }

    #[test]
    fn execute_day_matches_raw_simulation_when_debug_is_disabled() {
        let codex = load_codex(false, false);
        let mut raw_rng = SeededRng::new(SEED);
        let raw = simulate_day(&codex, day_context(&codex), &mut raw_rng);
        let raw_next = raw_rng.roll_range(0, 1_000_000);

        let mut executed_rng = SeededRng::new(SEED);
        let executed = execute_day(&codex, day_context(&codex), &mut executed_rng);
        let executed_next = executed_rng.roll_range(0, 1_000_000);

        assert_eq!(format!("{raw:#?}"), format!("{executed:#?}"));
        assert_eq!(raw_next, executed_next, "execution facade must not consume extra RNG");
    }

    /// A night-start battle is the one night battle where a submarine can be sunk; after a
    /// day battle a hit on her is a scratch (14% of her HP at most).
    #[test]
    fn a_night_start_battle_can_sink_a_submarine() {
        let codex = load_codex(false, false);
        let mut hunter = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 99);
        hunter.ship.api_taisen[0] = 100;
        hunter.slot_items =
            vec![slotitem_with_mst_id(first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar))];
        let mut submarine = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::SS), 50);
        submarine.ship.api_soukou[0] = 5;
        submarine.ship.api_nowhp = 30;
        submarine.ship.api_maxhp = 30;

        // Alone, and as 第2艦隊 of a combined fleet: the source zeroes the power for neither
        // in a night-start battle (`!NBonly && !isescort`).
        for combined in [false, true] {
            let sunk = (0..100).any(|seed| {
                let mut context = BattleContext::head_on(
                    BattleType::Normal,
                    true,
                    vec![hunter.clone()],
                    vec![submarine.clone()],
                );
                if combined {
                    context.combined = Some(CombinedSetup {
                        combined_type: CombinedType::SurfaceTaskForce,
                        escort_ships: vec![hunter.clone()],
                    });
                }
                let sp = execute_sp_midnight(&codex, context, &mut SeededRng::new(seed));
                assert_eq!(sp.main_deck.len(), usize::from(combined));
                sp.night.enemy[0].hp() <= 0
            });
            assert!(sunk, "combined: {combined}");
        }
    }

    #[test]
    fn execute_night_matches_raw_simulation_when_debug_is_disabled() {
        let codex = load_codex(false, false);
        let mut raw_rng = SeededRng::new(SEED);
        let raw = simulate_night(&codex, night_input(&codex), &mut raw_rng);
        let raw_next = raw_rng.roll_range(0, 1_000_000);

        let mut executed_rng = SeededRng::new(SEED);
        let executed = execute_night(&codex, night_input(&codex), &mut executed_rng);
        let executed_next = executed_rng.roll_range(0, 1_000_000);

        assert_eq!(format!("{raw:#?}"), format!("{executed:#?}"));
        assert_eq!(raw_next, executed_next, "execution facade must not consume extra RNG");
    }

    #[test]
    fn execute_day_matches_existing_debug_pipeline() {
        for (god_mode, one_hit_kill) in [(true, false), (false, true), (true, true)] {
            let codex = load_codex(god_mode, one_hit_kill);
            let mut manual_rng = SeededRng::new(SEED);
            let manual = apply_day_debug(
                simulate_day(&codex, day_context(&codex), &mut manual_rng),
                god_mode,
                one_hit_kill,
            );

            let mut executed_rng = SeededRng::new(SEED);
            let executed = execute_day(&codex, day_context(&codex), &mut executed_rng);

            assert_eq!(
                format!("{manual:#?}"),
                format!("{executed:#?}"),
                "day pipeline drift for god_mode={god_mode}, one_hit_kill={one_hit_kill}"
            );
        }
    }

    #[test]
    fn execute_night_matches_existing_debug_pipeline() {
        for (god_mode, one_hit_kill) in [(true, false), (false, true), (true, true)] {
            let codex = load_codex(god_mode, one_hit_kill);
            let mut manual_rng = SeededRng::new(SEED);
            let manual = apply_night_debug(
                simulate_night(&codex, night_input(&codex), &mut manual_rng),
                god_mode,
                one_hit_kill,
            );

            let mut executed_rng = SeededRng::new(SEED);
            let executed = execute_night(&codex, night_input(&codex), &mut executed_rng);

            assert_eq!(
                format!("{manual:#?}"),
                format!("{executed:#?}"),
                "night pipeline drift for god_mode={god_mode}, one_hit_kill={one_hit_kill}"
            );
        }
    }
}
