//! Event types for the debug-overlay architecture.
//!
//! The debug overlay derives events from HP diffs, applies transforms, and
//! reduces to a derived state. Events carry resolved damage so the reducer
//! can work without touching RNG.

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

/// Events derived by the debug overlay from HP diffs. Each event describes
/// a discrete effect that the [reducer](crate::reducer) processes to derive
/// final state.
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
}

/// A log of battle events, preserving emission order.
pub type EventLog = Vec<BattleEvent>;

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
}
