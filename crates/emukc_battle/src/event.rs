//! Event types for the owned-pass battle simulation architecture.
//!
//! Phase functions emit events alongside updated [`FleetState`](crate::state::FleetState).
//! Events carry resolved damage (RNG consumed during simulation), so the
//! [reducer](crate::reducer) can derive final state without touching RNG.
//!
//! Debug transforms (god mode, one hit kill) operate on `Vec<BattleEvent>` as
//! pure filters/maps — zero branching in simulation code.

use std::collections::BTreeSet;

/// Which side a ship belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Player's fleet.
    Friendly,
    /// Enemy fleet.
    Enemy,
}

/// Lightweight reference to a ship within a battle.
///
/// `(Side, index into the fleet slice)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShipRef(pub Side, pub usize);

impl ShipRef {
    /// Returns the side this ship belongs to.
    pub fn side(self) -> Side {
        self.0
    }

    /// Returns the fleet index.
    pub fn index(self) -> usize {
        self.1
    }
}

/// Which battle phase produced an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Aerial combat (航空戦).
    Kouku,
    /// Opening ASW (開幕対潜).
    OpeningAsw,
    /// Opening torpedo (開幕雷撃).
    OpeningTorpedo,
    /// First shelling round (砲撃戦1).
    Shelling1,
    /// Second shelling round (砲撃戦2).
    Shelling2,
    /// Closing torpedo (雷撃戦).
    ClosingTorpedo,
    /// Night battle (夜戦).
    Night,
}

/// Events emitted by simulation phases. Each event describes a discrete
/// effect that the [reducer](crate::reducer) processes to derive final state.
///
/// Events carry resolved values — RNG is consumed during simulation, and the
/// results (including proportional damage amounts) are baked into the event.
#[derive(Debug, Clone)]
pub enum BattleEvent {
    /// Raw damage applied to a ship. `raw` is the calculated damage before
    /// clamping; `dealt` is the actual HP subtracted (after clamping and
    /// sinking protection).
    Damage {
        /// The ship taking damage.
        target: ShipRef,
        /// Damage before clamping/protection.
        raw: i64,
        /// Actual HP subtracted.
        dealt: i64,
        /// Which phase this occurred in.
        phase: Phase,
    },

    /// Sinking-protection proportional damage (割合ダメージ). The RNG draw
    /// was already consumed by the simulation; `amount` is the final value.
    ProportionalDamage {
        /// The ship taking proportional damage.
        target: ShipRef,
        /// Pre-computed proportional damage amount.
        amount: i64,
    },

    /// A ship has been sunk (HP reached 0).
    Sunk {
        /// The ship that sank.
        target: ShipRef,
    },

    /// A ship was targeted by an attack. Used for MVP / damage-dealt tracking.
    Targeted {
        /// The attacking ship.
        attacker: ShipRef,
        /// The ship being attacked.
        target: ShipRef,
        /// Which phase this occurred in.
        phase: Phase,
    },

    /// Marks the start of a battle phase.
    PhaseStart {
        /// The phase starting.
        phase: Phase,
    },

    /// Marks the end of a battle phase.
    PhaseEnd {
        /// The phase ending.
        phase: Phase,
    },

    /// Wraps aerial combat results for packet assembly.
    AirCombat {
        /// The aerial combat packet data.
        kouku: crate::types::BattleKouku,
    },

    /// Wraps torpedo salvo data for packet assembly.
    TorpedoSalvo {
        /// Whether this is an opening torpedo attack.
        is_opening: bool,
        /// Opening torpedo attack data (if opening).
        opening: Option<crate::types::BattleOpeningAttack>,
        /// Closing torpedo (raigeki) data (if closing).
        raigeki: Option<crate::types::BattleRaigeki>,
    },

    /// Wraps shelling exchange data for packet assembly.
    ShellingExchange {
        /// Which shelling round (1 or 2).
        round: u8,
        /// The hougeki packet data.
        hougeki: crate::types::BattleHougeki,
    },
}

/// A log of battle events, preserving emission order.
pub type EventLog = Vec<BattleEvent>;

/// Collect all enemy ship indices that received a `Damage` event.
///
/// Used by the `one_hit_kill` transform to determine which enemies were
/// targeted — unhit enemies need synthesized `Sunk` events.
pub fn targeted_enemy_indices(events: &[BattleEvent]) -> BTreeSet<usize> {
    events
        .iter()
        .filter_map(|e| match e {
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, idx),
                ..
            } => Some(*idx),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_ref_distinguishes_sides() {
        let f = ShipRef(Side::Friendly, 0);
        let e = ShipRef(Side::Enemy, 1);
        assert_eq!(f.side(), Side::Friendly);
        assert_eq!(e.side(), Side::Enemy);
        assert_eq!(f.index(), 0);
        assert_eq!(e.index(), 1);
        assert_ne!(f, e);
    }

    #[test]
    fn empty_event_log_is_valid() {
        let log: EventLog = vec![];
        assert!(log.is_empty());
    }

    #[test]
    fn targeted_enemy_indices_collects_from_damage_events() {
        let events = vec![
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 50,
                dealt: 50,
                phase: Phase::Shelling1,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, 0),
                raw: 10,
                dealt: 10,
                phase: Phase::Shelling1,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 2),
                raw: 30,
                dealt: 30,
                phase: Phase::Shelling1,
            },
        ];
        let indices = targeted_enemy_indices(&events);
        assert_eq!(indices, BTreeSet::from([0, 2]));
    }

    #[test]
    fn targeted_enemy_indices_empty_when_no_enemy_damage() {
        let events = vec![
            BattleEvent::PhaseStart {
                phase: Phase::Kouku,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, 0),
                raw: 10,
                dealt: 10,
                phase: Phase::Kouku,
            },
        ];
        let indices = targeted_enemy_indices(&events);
        assert!(indices.is_empty());
    }
}
