//! Who fires at whom in a phase: which of each side's ships fire, and which of
//! the other side's they fire at.
//!
//! Every phase works in slices of the two fleets, and every index it writes must
//! be a position in the whole fleet. A [`Volley`] holds both ranges, so a phase
//! takes its slices from it and lifts its indices through it; the rules for
//! which deck of a combined fleet takes part live in one table here
//! ([`Volleys::opening`]) and in the shelling round tables of the orchestrator.
//!
//! Every range starts at a deck's first ship (index 0, or where the escort deck
//! begins), so the ship at index 0 of a slice is that deck's flagship: the
//! phases' `idx == 0` checks rely on this.

use std::ops::Range;

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
        (&mut attacking[self.attackers.clone()], &mut defending[self.targets.clone()])
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
            BattlePhaseKind::ClosingTorpedo => escort_start(enemy).unwrap_or(0)..enemy.len(),
            _ => 0..enemy.len(),
        };
        Self::between(
            escort_start(friendly).unwrap_or(0)..friendly.len(),
            enemy_fires,
            0..enemy.len(),
        )
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
/// the escort fleet of an enemy one. `None` for a single fleet.
pub(crate) fn escort_start(ships: &[BattleRuntimeShip]) -> Option<usize> {
    ships.iter().position(BattleRuntimeShip::is_escort_deck)
}
