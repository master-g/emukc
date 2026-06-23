//! Pure reducer: derives final battle state from an event log.
//!
//! Processes events sequentially to compute final HP and sunk status.
//! No RNG — all RNG-dependent values (including proportional damage
//! amounts) are carried in the events themselves.
//!
//! The reducer exists to support [debug transforms](crate::transforms):
//! transforms modify the event stream, then the reducer derives the
//! transformed result. Without transforms, the primary simulation state
//! already tracks HP — the reducer is for the transform-applied path.

use crate::event::{BattleEvent, Side};

/// Initial HP values for each ship, captured at battle start.
#[derive(Debug, Clone)]
pub struct InitialState {
    /// Friendly ship HP at battle start, indexed by fleet position.
    pub friendly_hp: Vec<i64>,
    /// Enemy ship HP at battle start, indexed by fleet position.
    pub enemy_hp: Vec<i64>,
}

impl InitialState {
    /// Create from explicit HP arrays.
    pub fn new(friendly_hp: Vec<i64>, enemy_hp: Vec<i64>) -> Self {
        Self {
            friendly_hp,
            enemy_hp,
        }
    }
}

/// Final battle state derived from processing an event log.
#[derive(Debug, Clone)]
pub struct DerivedState {
    /// Final friendly HP values.
    pub friendly_hp: Vec<i64>,
    /// Final enemy HP values.
    pub enemy_hp: Vec<i64>,
    /// Whether each friendly ship is sunk.
    pub friendly_sunk: Vec<bool>,
    /// Whether each enemy ship is sunk.
    pub enemy_sunk: Vec<bool>,
}

impl DerivedState {
    /// Returns true if any ship on the given side is still alive.
    pub fn any_alive(&self, side: Side) -> bool {
        match side {
            Side::Friendly => self.friendly_sunk.iter().any(|&s| !s),
            Side::Enemy => self.enemy_sunk.iter().any(|&s| !s),
        }
    }

    /// Returns the HP of a specific ship, or 0 if out of bounds.
    pub fn hp(&self, side: Side, index: usize) -> i64 {
        match side {
            Side::Friendly => self.friendly_hp.get(index).copied().unwrap_or(0),
            Side::Enemy => self.enemy_hp.get(index).copied().unwrap_or(0),
        }
    }
}

/// Process an event log against initial state to derive final state.
///
/// This is a pure function — same inputs always produce the same output.
/// No RNG is consumed.
pub fn reduce(events: &[BattleEvent], initial: &InitialState) -> DerivedState {
    let friendly_count = initial.friendly_hp.len();
    let enemy_count = initial.enemy_hp.len();

    let mut hp: Vec<Vec<i64>> = vec![initial.friendly_hp.clone(), initial.enemy_hp.clone()];
    let mut sunk: Vec<Vec<bool>> = vec![vec![false; friendly_count], vec![false; enemy_count]];

    let side_idx = |side: Side| match side {
        Side::Friendly => 0,
        Side::Enemy => 1,
    };

    for event in events {
        match event {
            BattleEvent::Damage {
                target,
                dealt,
                ..
            } => {
                let s = side_idx(target.side());
                let i = target.index();
                if !sunk[s].get(i).copied().unwrap_or(true)
                    && let Some(h) = hp[s].get_mut(i)
                {
                    *h = (*h - *dealt).max(0);
                    if *h == 0 {
                        sunk[s][i] = true;
                    }
                }
            }

            BattleEvent::ProportionalDamage {
                target,
                amount,
            } => {
                let s = side_idx(target.side());
                let i = target.index();
                if !sunk[s].get(i).copied().unwrap_or(true)
                    && let Some(h) = hp[s].get_mut(i)
                {
                    *h = (*h - *amount).max(0);
                    if *h == 0 {
                        sunk[s][i] = true;
                    }
                }
            }

            BattleEvent::Sunk {
                target,
            } => {
                let s = side_idx(target.side());
                let i = target.index();
                if sunk[s].get_mut(i).is_some() {
                    sunk[s][i] = true;
                    if let Some(h) = hp[s].get_mut(i) {
                        *h = 0;
                    }
                }
            }
        }
    }

    DerivedState {
        friendly_hp: hp[0].clone(),
        enemy_hp: hp[1].clone(),
        friendly_sunk: sunk[0].clone(),
        enemy_sunk: sunk[1].clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Phase, ShipRef, Side};

    fn initial(hp_f: &[i64], hp_e: &[i64]) -> InitialState {
        InitialState::new(hp_f.to_vec(), hp_e.to_vec())
    }

    #[test]
    fn empty_events_preserve_initial_hp() {
        let init = initial(&[30, 40], &[50, 60]);
        let state = reduce(&[], &init);
        assert_eq!(state.friendly_hp, vec![30, 40]);
        assert_eq!(state.enemy_hp, vec![50, 60]);
        assert!(!state.friendly_sunk[0]);
    }

    #[test]
    fn single_damage_reduces_hp() {
        let init = initial(&[30], &[50]);
        let events = vec![BattleEvent::Damage {
            target: ShipRef(Side::Enemy, 0),
            raw: 20,
            dealt: 20,
            phase: Phase::Shelling1,
        }];
        let state = reduce(&events, &init);
        assert_eq!(state.enemy_hp, vec![30]);
        assert!(!state.enemy_sunk[0]);
    }

    #[test]
    fn lethal_damage_sinks() {
        let init = initial(&[30], &[50]);
        let events = vec![BattleEvent::Damage {
            target: ShipRef(Side::Enemy, 0),
            raw: 999,
            dealt: 50,
            phase: Phase::Shelling1,
        }];
        let state = reduce(&events, &init);
        assert_eq!(state.enemy_hp, vec![0]);
        assert!(state.enemy_sunk[0]);
    }

    #[test]
    fn damage_on_sunk_is_noop() {
        let init = initial(&[30], &[50]);
        let events = vec![
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 50,
                dealt: 50,
                phase: Phase::Shelling1,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 30,
                dealt: 30,
                phase: Phase::Shelling2,
            },
        ];
        let state = reduce(&events, &init);
        assert_eq!(state.enemy_hp, vec![0]);
        assert!(state.enemy_sunk[0]);
    }

    #[test]
    fn multiple_damage_cumulative() {
        let init = initial(&[30], &[50]);
        let events = vec![
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 20,
                dealt: 20,
                phase: Phase::Shelling1,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 15,
                dealt: 15,
                phase: Phase::Shelling2,
            },
        ];
        let state = reduce(&events, &init);
        assert_eq!(state.enemy_hp, vec![15]);
    }

    #[test]
    fn sunk_event_sets_hp_zero() {
        let init = initial(&[30], &[50]);
        let events = vec![BattleEvent::Sunk {
            target: ShipRef(Side::Enemy, 0),
        }];
        let state = reduce(&events, &init);
        assert_eq!(state.enemy_hp, vec![0]);
        assert!(state.enemy_sunk[0]);
    }

    #[test]
    fn proportional_damage_reduces_hp() {
        let init = initial(&[30], &[50]);
        let events = vec![BattleEvent::ProportionalDamage {
            target: ShipRef(Side::Friendly, 0),
            amount: 10,
        }];
        let state = reduce(&events, &init);
        assert_eq!(state.friendly_hp, vec![20]);
    }

    #[test]
    fn any_alive_correct_after_events() {
        let init = initial(&[30], &[50, 40]);
        let events = vec![BattleEvent::Sunk {
            target: ShipRef(Side::Enemy, 0),
        }];
        let state = reduce(&events, &init);
        assert!(state.any_alive(Side::Enemy), "enemy 1 still alive");
        let events2 = vec![
            BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, 0),
            },
            BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, 1),
            },
        ];
        let state2 = reduce(&events2, &init);
        assert!(!state2.any_alive(Side::Enemy), "all enemies sunk");
    }

    #[test]
    fn friendly_and_enemy_hp_independent() {
        let init = initial(&[30, 20], &[50]);
        let events = vec![
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, 0),
                raw: 10,
                dealt: 10,
                phase: Phase::Shelling1,
            },
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                raw: 20,
                dealt: 20,
                phase: Phase::Shelling1,
            },
        ];
        let state = reduce(&events, &init);
        assert_eq!(state.friendly_hp, vec![20, 20]);
        assert_eq!(state.enemy_hp, vec![30]);
    }
}
