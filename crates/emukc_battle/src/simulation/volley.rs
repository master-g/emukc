//! Who fires at whom in a phase: which of each side's ships fire, and which of
//! the other side's they fire at.
//!
//! Every phase works in slices of the two fleets, and every index it writes must
//! be a position in the whole fleet. A [`Volley`] holds both ranges, so a phase
//! takes its slices from it and lifts its indices through it. Which deck of a
//! combined fleet takes part, and which deck the other side fires at, is written
//! here and nowhere else: [`Volleys::opening`] for the opening ASW and the two
//! torpedo phases, and one constructor for each shape of shelling round. When
//! each round comes, and into which packet slot, is the orchestrator's.
//!
//! Every range starts at a deck's first ship (index 0, or where the escort deck
//! begins), so the ship at index 0 of a slice is that deck's flagship: the
//! phases' `idx == 0` checks rely on this, and [`Volley::split`] asserts it in
//! debug builds.

use std::ops::Range;

use crate::combined::CombinedFleetRole;
use crate::config::BattlePhaseKind;
use crate::types::BattleRuntimeShip;

/// One side's fire in a phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Volley {
    /// The ships that fire, in their own fleet.
    pub(crate) attackers: Range<usize>,
    /// The ships they fire at, in the other fleet.
    pub(crate) targets: Range<usize>,
}

impl Volley {
    /// The attacking and the fired-at slices of the two fleets.
    pub(crate) fn split<'a>(
        &self,
        attacking: &'a mut [BattleRuntimeShip],
        defending: &'a mut [BattleRuntimeShip],
    ) -> (&'a mut [BattleRuntimeShip], &'a mut [BattleRuntimeShip]) {
        debug_assert!(starts_a_deck(attacking, self.attackers.start), "{self:?}");
        debug_assert!(starts_a_deck(defending, self.targets.start), "{self:?}");
        (&mut attacking[self.attackers.clone()], &mut defending[self.targets.clone()])
    }

    /// The ships of `fleet` that fire.
    pub(crate) fn attackers_in<'a>(
        &self,
        fleet: &'a [BattleRuntimeShip],
    ) -> &'a [BattleRuntimeShip] {
        &fleet[self.attackers.clone()]
    }

    /// The ships of `fleet` that are fired at.
    pub(crate) fn targets_in<'a>(&self, fleet: &'a [BattleRuntimeShip]) -> &'a [BattleRuntimeShip] {
        &fleet[self.targets.clone()]
    }

    /// The position in its whole fleet of the attacker at `idx` of the slice.
    pub(crate) fn attacker_at(&self, idx: usize) -> usize {
        self.attackers.start + idx
    }

    /// The position in its whole fleet of the target at `idx` of the slice.
    pub(crate) fn target_at(&self, idx: usize) -> usize {
        self.targets.start + idx
    }
}

/// Both sides' fire in a phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Volleys {
    pub(crate) friendly: Volley,
    pub(crate) enemy: Volley,
}

impl Volleys {
    /// The opening ASW, the opening torpedo and the closing torpedo
    /// (`kcsimcombined.js` `69097abc` gives them `alive1C`): a friendly combined
    /// fleet fights them with 第2艦隊 alone, which fires at the whole enemy and is
    /// the only part the enemy fires at. Of an enemy combined fleet only the escort
    /// fleet closes with torpedoes. (Upstream lets a handful of ships open-torpedo
    /// from the main deck, `canOpTorpMain` in `kcsim.js:1811`, which is not
    /// modelled here.)
    pub(crate) fn opening(
        phase: BattlePhaseKind,
        friendly: &[BattleRuntimeShip],
        enemy: &[BattleRuntimeShip],
    ) -> Self {
        let enemy_fires = match phase {
            BattlePhaseKind::ClosingTorpedo => escort_start(enemy)..enemy.len(),
            _ => 0..enemy.len(),
        };
        Self::between(escort_start(friendly)..friendly.len(), enemy_fires, 0..enemy.len())
    }

    /// A shelling round between two single fleets: all of each fires at all of
    /// the other.
    pub(crate) fn whole(friendly: &[BattleRuntimeShip], enemy: &[BattleRuntimeShip]) -> Self {
        Self::between(0..friendly.len(), 0..enemy.len(), 0..enemy.len())
    }

    /// A shelling round of a friendly combined fleet: one `deck` fires at the
    /// whole enemy, which fires back at that deck (`kcsimcombined.js` `69097abc`,
    /// `shellRange`/`shellOrder` pass that deck's `alive1f`).
    pub(crate) fn friendly_deck_round(
        deck: CombinedFleetRole,
        friendly: &[BattleRuntimeShip],
        enemy: &[BattleRuntimeShip],
    ) -> Self {
        let escort = escort_start(friendly);
        let friendly_deck = match deck {
            CombinedFleetRole::Main => 0..escort,
            CombinedFleetRole::Escort => escort..friendly.len(),
        };
        Self::between(friendly_deck, 0..enemy.len(), 0..enemy.len())
    }

    /// Shelling round `round` (0, 1 or 2) of a single fleet against an enemy
    /// combined fleet: against the escort fleet, then the main fleet, then the
    /// battleship round, in which the main fleet alone fires and both its decks
    /// are fired at (`kcsimcombined.js` `69097abc`, shelling 3).
    pub(crate) fn enemy_combined_round(
        round: usize,
        friendly: &[BattleRuntimeShip],
        enemy: &[BattleRuntimeShip],
    ) -> Self {
        let escort = escort_start(enemy);
        let (fires, fired_at) = match round {
            0 => (escort..enemy.len(), escort..enemy.len()),
            1 => (0..escort, 0..escort),
            _ => (0..escort, 0..enemy.len()),
        };
        Self::between(0..friendly.len(), fires, fired_at)
    }

    /// `friendly` fires at `enemy_targets`, and `enemy` fires back at `friendly`.
    pub(crate) fn between(
        friendly: Range<usize>,
        enemy: Range<usize>,
        enemy_targets: Range<usize>,
    ) -> Self {
        Self {
            friendly: Volley {
                attackers: friendly.clone(),
                targets: enemy_targets,
            },
            enemy: Volley {
                attackers: enemy,
                targets: friendly,
            },
        }
    }
}

/// Where a fleet's escort deck begins: 第2艦隊 of a friendly combined fleet, or
/// the escort fleet of an enemy one. 0 for a single fleet, which is one deck.
fn escort_start(ships: &[BattleRuntimeShip]) -> usize {
    ships.iter().position(BattleRuntimeShip::is_escort_deck).unwrap_or(0)
}

/// Whether a range starting `at` starts at a deck's first ship, or is empty.
fn starts_a_deck(fleet: &[BattleRuntimeShip], at: usize) -> bool {
    at == 0 || at == fleet.len() || at == escort_start(fleet)
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;

    use super::*;
    use crate::combined::CombinedType;
    use crate::state::BattleState;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship};
    use crate::types::{BattleContext, BattleType, CombinedSetup};

    /// Fleets of destroyers, tagged as `BattleState` tags them.
    fn state(main: usize, escort: usize, enemy_main: usize, enemy_escort: usize) -> BattleState {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ships = |n: usize| (0..n).map(|_| sample_ship(&codex, dd, 50)).collect::<Vec<_>>();
        let mut context =
            BattleContext::head_on(BattleType::Normal, true, ships(main), ships(enemy_main));
        context.enemy_escort_ships = ships(enemy_escort);
        context.combined = (escort > 0).then(|| CombinedSetup {
            combined_type: CombinedType::CarrierTaskForce,
            escort_ships: ships(escort),
        });
        BattleState::from_context(context)
    }

    type Ranges = (Range<usize>, Range<usize>, Range<usize>, Range<usize>);

    /// (friendly fires, at, enemy fires, at).
    fn ranges(volleys: Volleys) -> Ranges {
        let Volleys {
            friendly,
            enemy,
        } = volleys;
        (friendly.attackers, friendly.targets, enemy.attackers, enemy.targets)
    }

    fn opening(state: &BattleState, phase: BattlePhaseKind) -> Ranges {
        ranges(Volleys::opening(phase, &state.friendly, &state.enemy))
    }

    #[test]
    fn the_opening_phases_follow_the_fleet_shape() {
        use BattlePhaseKind::{ClosingTorpedo, OpeningAsw, OpeningTorpedo};
        let single = state(6, 0, 6, 0);
        for phase in [OpeningAsw, OpeningTorpedo, ClosingTorpedo] {
            assert_eq!(opening(&single, phase), (0..6, 0..6, 0..6, 0..6), "{phase:?}");
        }
        // 第2艦隊 alone fires and is fired at, wherever 第1艦隊 ends.
        let combined = state(2, 6, 6, 0);
        for phase in [OpeningAsw, OpeningTorpedo, ClosingTorpedo] {
            assert_eq!(opening(&combined, phase), (2..8, 0..6, 0..6, 2..8), "{phase:?}");
        }
        // Both enemy decks fire and are fired at, but only the escort fleet closes.
        let enemy_combined = state(6, 0, 4, 6);
        for phase in [OpeningAsw, OpeningTorpedo] {
            assert_eq!(opening(&enemy_combined, phase), (0..6, 0..10, 0..10, 0..6), "{phase:?}");
        }
        assert_eq!(opening(&enemy_combined, ClosingTorpedo), (0..6, 0..10, 4..10, 0..6));
    }

    #[test]
    fn the_shelling_rounds_follow_the_fleet_shape() {
        let single = state(6, 0, 6, 0);
        let whole = Volleys::whole(&single.friendly, &single.enemy);
        assert_eq!(ranges(whole), (0..6, 0..6, 0..6, 0..6));
        // A combined fleet's round is one deck's, and the enemy fires back at that deck.
        let combined = state(2, 6, 6, 0);
        let round = |deck| Volleys::friendly_deck_round(deck, &combined.friendly, &combined.enemy);
        assert_eq!(ranges(round(CombinedFleetRole::Main)), (0..2, 0..6, 0..6, 0..2));
        assert_eq!(ranges(round(CombinedFleetRole::Escort)), (2..8, 0..6, 0..6, 2..8));
        // Against the escort fleet, the main fleet, then the main fleet against both.
        let enemy_combined = state(6, 0, 4, 6);
        let round =
            |n| Volleys::enemy_combined_round(n, &enemy_combined.friendly, &enemy_combined.enemy);
        assert_eq!(ranges(round(0)), (0..6, 4..10, 4..10, 0..6));
        assert_eq!(ranges(round(1)), (0..6, 0..4, 0..4, 0..6));
        assert_eq!(ranges(round(2)), (0..6, 0..10, 0..4, 0..6));
    }
}
