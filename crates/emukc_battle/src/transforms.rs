//! Pure event-stream transforms for debug features.
//!
//! Each transform is a `fn(Vec<BattleEvent>) -> Vec<BattleEvent>` — a pure
//! filter or map over the event stream. Simulation code never branches on
//! debug flags; transforms are applied between simulation and reduction.
//!
//! Adding a new debug feature means adding a new transform function, not
//! modifying any simulation code.

use std::collections::BTreeSet;

use crate::event::{BattleEvent, EventLog, ShipRef, Side};

/// God mode: zeroes all damage to friendly ships by filtering out their
/// `Damage`, `ProportionalDamage`, **and `Sunk`** events. Friendly ships
/// take zero HP loss and any friendly that sank during simulation is
/// revived to its entry HP.
///
/// Note: targeting, torpedo eligibility, and other HP-gated checks still run
/// on real HP during simulation. This means friendly ships may enter chūha
/// during simulation, affecting torpedo phase participation — but the final
/// derived HP shows full health. This is acceptable for debug use.
pub fn god_mode_transform(events: EventLog) -> EventLog {
    events
        .into_iter()
        .filter(|e| {
            !matches!(
                e,
                BattleEvent::Damage {
                    target: ShipRef(Side::Friendly, _),
                    ..
                } | BattleEvent::ProportionalDamage {
                    target: ShipRef(Side::Friendly, _),
                    ..
                } | BattleEvent::Sunk {
                    target: ShipRef(Side::Friendly, _),
                }
            )
        })
        .collect()
}

/// One hit kill: every enemy hit becomes lethal, and unhit enemies are
/// force-sunk so ALL enemies are dead in the derived state.
///
/// Enemy `Damage` events are replaced with `Sunk` events. After processing
/// all events, any enemy index that received zero `Damage` events gets a
/// synthesized `Sunk` event.
///
/// Note: because simulation ran without this transform, targeting consumed
/// RNG on the original (alive) enemy set. Post-hoc sinking does not change
/// which attacks were made — it only changes the outcome.
pub fn one_hit_kill_transform(events: EventLog, enemy_count: usize) -> EventLog {
    let mut sunk_enemies: BTreeSet<usize> = BTreeSet::new();
    let mut result: EventLog = Vec::with_capacity(events.len() + enemy_count);

    for event in events {
        match &event {
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, idx),
                ..
            } => {
                sunk_enemies.insert(*idx);
                result.push(BattleEvent::Sunk {
                    target: ShipRef(Side::Enemy, *idx),
                });
            }
            _ => result.push(event),
        }
    }

    // Synthesize Sunk events for enemies that were never targeted.
    for idx in 0..enemy_count {
        if !sunk_enemies.contains(&idx) {
            result.push(BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, idx),
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Phase, ShipRef, Side};

    fn damage(target: ShipRef, dealt: i64) -> BattleEvent {
        BattleEvent::Damage {
            target,
            raw: dealt,
            dealt,
            phase: Phase::Shelling1,
        }
    }

    fn sunk(target: ShipRef) -> BattleEvent {
        BattleEvent::Sunk {
            target,
        }
    }

    fn prop_dmg(target: ShipRef, amount: i64) -> BattleEvent {
        BattleEvent::ProportionalDamage {
            target,
            amount,
        }
    }

    #[test]
    fn god_mode_filters_friendly_damage() {
        let events = vec![
            damage(ShipRef(Side::Friendly, 0), 30),
            damage(ShipRef(Side::Enemy, 0), 50),
            damage(ShipRef(Side::Friendly, 1), 10),
        ];
        let result = god_mode_transform(events);
        // Only enemy damage survives.
        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            BattleEvent::Damage {
                target: ShipRef(Side::Enemy, 0),
                ..
            }
        ));
    }

    #[test]
    fn god_mode_filters_friendly_proportional_damage() {
        let events =
            vec![prop_dmg(ShipRef(Side::Friendly, 0), 5), prop_dmg(ShipRef(Side::Enemy, 0), 10)];
        let result = god_mode_transform(events);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn god_mode_preserves_non_damage_events() {
        let events = vec![
            sunk(ShipRef(Side::Enemy, 0)),
            BattleEvent::PhaseStart {
                phase: Phase::Kouku,
            },
            damage(ShipRef(Side::Friendly, 0), 30),
        ];
        let result = god_mode_transform(events);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn god_mode_filters_friendly_sunk() {
        let events = vec![sunk(ShipRef(Side::Friendly, 0)), sunk(ShipRef(Side::Enemy, 0))];
        let result = god_mode_transform(events);
        // Friendly Sunk dropped, enemy Sunk preserved.
        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, 0)
            }
        ));
    }

    #[test]
    fn god_mode_revives_sunk_friendly_in_compose() {
        // Friendly sunk + enemy alive → god_mode revives friendly,
        // one_hit_kill sinks enemy.
        let events = vec![damage(ShipRef(Side::Friendly, 0), 50), sunk(ShipRef(Side::Friendly, 0))];
        let result = one_hit_kill_transform(god_mode_transform(events), 1);
        // No friendly events survive god_mode. Enemy 0 gets synthesized Sunk.
        assert!(result.iter().all(|e| !matches!(
            e,
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, _),
                ..
            } | BattleEvent::Sunk {
                target: ShipRef(Side::Friendly, _)
            }
        )));
        assert!(result.iter().any(|e| matches!(
            e,
            BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, 0)
            }
        )));
    }

    #[test]
    fn one_hit_kill_converts_enemy_damage_to_sunk() {
        let events = vec![
            damage(ShipRef(Side::Enemy, 0), 50),
            damage(ShipRef(Side::Friendly, 0), 10),
            damage(ShipRef(Side::Enemy, 1), 30),
        ];
        let result = one_hit_kill_transform(events, 3);
        // Both targeted enemies (0, 1) get Sunk. Enemy 2 (unhit) gets Sunk too.
        let sunk_count = result.iter().filter(|e| matches!(e, BattleEvent::Sunk { .. })).count();
        assert_eq!(sunk_count, 3);
    }

    #[test]
    fn one_hit_kill_preserves_friendly_damage() {
        let events =
            vec![damage(ShipRef(Side::Enemy, 0), 50), damage(ShipRef(Side::Friendly, 0), 10)];
        let result = one_hit_kill_transform(events, 1);
        // Friendly damage survives as-is.
        assert!(result.iter().any(|e| matches!(
            e,
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, 0),
                ..
            }
        )));
    }

    #[test]
    fn one_hit_kill_sinks_unhit_enemies() {
        // 3 enemies, none receive Damage events.
        let events = vec![damage(ShipRef(Side::Friendly, 0), 10)];
        let result = one_hit_kill_transform(events, 3);
        let sunk_count = result.iter().filter(|e| matches!(e, BattleEvent::Sunk { .. })).count();
        assert_eq!(sunk_count, 3, "all 3 enemies should get Sunk");
    }

    #[test]
    fn one_hit_kill_empty_enemy_set() {
        let events = vec![damage(ShipRef(Side::Friendly, 0), 10)];
        let result = one_hit_kill_transform(events, 0);
        // No enemies to sink, friendly damage preserved.
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn composed_transforms_god_mode_then_one_hit_kill() {
        let events =
            vec![damage(ShipRef(Side::Friendly, 0), 30), damage(ShipRef(Side::Enemy, 0), 50)];
        let result = one_hit_kill_transform(god_mode_transform(events), 1);
        // Friendly damage removed by god_mode, enemy sunk by one_hit_kill.
        assert!(result.iter().all(|e| !matches!(
            e,
            BattleEvent::Damage {
                target: ShipRef(Side::Friendly, _),
                ..
            }
        )));
        assert!(result.iter().any(|e| matches!(
            e,
            BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, 0)
            }
        )));
    }
}
