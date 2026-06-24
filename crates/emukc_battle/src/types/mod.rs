//! Battle types — domain enums, API packet structs, and runtime state.

pub(crate) mod domain;
pub(crate) mod packet;
mod runtime;

// Re-export everything that was previously in types.rs
pub use domain::{AirState, BattleType, EngagementType};
pub(crate) use domain::{
    AirstrikeOutput, AttackCapability, BattlePhase, NightBattleParams, ShellingParams, TargetClass,
    TorpedoAttackerSide, TorpedoHit,
};
pub use packet::SiListId;
pub use packet::{
    BattleHougeki, BattleKouku, BattleKoukuStage1, BattleKoukuStage2, BattleKoukuStage3,
    BattleNightHougeki, BattleOpeningAttack, BattleRaigeki, DamageCell,
};
pub use runtime::{
    BattleContext, BattleOutcome, BattlePacket, BattleRuntimeShip, BattleShipInput,
    BattleSimulation, NightBattleInput, NightBattlePacket, NightBattleSimulation,
};

#[cfg(test)]
mod tests {
    use super::domain::*;
    use super::packet::*;
    use crate::test_utils::make_test_ship_ctx;

    // ── AirState tests ──────────────────────────────────────────────

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

    // ── Sinking protection tests ────────────────────────────────────

    #[test]
    fn sinking_protection_saves_non_taiha_ship_in_sortie() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(30, 30, 30, 40, true, true);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert!(ship.hp() >= 1, "ship must survive with sinking protection");
        assert!(effective < 30, "effective damage must be less than current HP");
        assert_eq!(raw, 999, "raw should show full input damage");
    }

    #[test]
    fn flagship_always_survives_even_when_taiha() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(5, 5, 5, 40, true, true);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 0);
        assert!(ship.hp() >= 1, "flagship must always survive");
        assert!(effective < 5);
        assert_eq!(raw, 999);
    }

    #[test]
    fn taiha_advance_ship_can_be_sunk() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(5, 5, 5, 40, true, true);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert_eq!(ship.hp(), 0, "taiha-advance ship should be sunk");
        assert_eq!(effective, 5);
        assert_eq!(raw, 999);
    }

    #[test]
    fn practice_never_triggers_sinking_protection() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(30, 30, 30, 40, true, false);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert_eq!(ship.hp(), 0, "practice uses normal damage clamping");
        assert_eq!(effective, 30);
        assert_eq!(raw, 999);
    }

    #[test]
    fn enemy_ships_never_get_sinking_protection() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(30, 30, 30, 40, false, true);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 0);
        assert_eq!(ship.hp(), 0, "enemy ships should be sinkable");
        assert_eq!(effective, 30);
        assert_eq!(raw, 999);
    }

    #[test]
    fn flagship_is_always_protected_from_sinking() {
        let mut ship = make_test_ship_ctx(10, 10, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert!(effective > 0, "flagship should take proportional damage");
        assert!(ship.current_hp > 0, "flagship must survive");
        assert!(ship.current_hp < ship.entry_hp, "should be proportional, not full damage");
        assert_eq!(raw, 100, "raw should show full input");
    }

    #[test]
    fn flagship_at_1hp_survives_lethal_damage() {
        let mut ship = make_test_ship_ctx(1, 5, 1, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 0, "at 1 HP, protection reduces damage to 0");
        assert_eq!(ship.current_hp, 1, "flagship must survive");
        assert_eq!(raw, 100);
    }

    #[test]
    fn non_taiha_ship_is_protected_from_sinking() {
        let mut ship = make_test_ship_ctx(10, 20, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 2);
        assert!(effective > 0, "non-taiha ship should take proportional damage");
        assert!(ship.current_hp > 0, "non-taiha ship must survive");
        assert_eq!(raw, 100);
    }

    #[test]
    fn taiha_non_flagship_can_be_sunk() {
        let entry_hp = 5;
        let max_hp = 30;
        let mut ship = make_test_ship_ctx(entry_hp, entry_hp, entry_hp, max_hp, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 2);
        assert_eq!(ship.current_hp, 0, "taiha non-flagship should be sunk");
        assert_eq!(effective, 5);
        assert_eq!(raw, 100);
    }

    #[test]
    fn protection_uses_entry_hp_not_current_hp() {
        let max_hp = 40;
        let mut ship = make_test_ship_ctx(10, 30, 10, max_hp, true, true);
        let mut rng = crate::random::SeededRng::new(123);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 1);
        assert!(effective > 0);
        assert!(ship.current_hp > 0, "should survive due to protection");
        assert!(
            ship.current_hp <= 30,
            "remaining HP should be based on entry_hp (30), not current_hp (10)"
        );
        assert_eq!(raw, 100);
    }

    #[test]
    fn enemy_ships_get_no_protection() {
        let mut ship = make_test_ship_ctx(1, 1, 1, 30, false, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 1, "enemy ship should take full effective damage");
        assert_eq!(raw, 100, "raw should show overkill");
        assert_eq!(ship.current_hp, 0, "enemy ship should be sunk");
    }

    #[test]
    fn practice_ships_get_no_protection() {
        let mut ship = make_test_ship_ctx(1, 1, 1, 30, true, false);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 1, "practice ship should take full effective damage");
        assert_eq!(raw, 100, "raw should show overkill");
        assert_eq!(ship.current_hp, 0, "practice ship should be sunk");
    }

    #[test]
    fn overkill_shows_raw_damage() {
        let mut ship = make_test_ship_ctx(5, 5, 5, 30, false, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(raw, 100, "raw should show full input damage");
        assert_eq!(effective, 5, "effective capped to current HP");
        assert_eq!(ship.current_hp, 0, "ship should be sunk");
    }

    #[test]
    fn protection_shows_raw_but_reduces_hp_proportionally() {
        let mut ship = make_test_ship_ctx(10, 10, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 200, 0);
        assert_eq!(raw, 200, "raw should show full lethal input");
        assert!(effective < 10, "effective should be proportional, not lethal");
        assert!(ship.current_hp > 0, "flagship must survive");
    }

    // ── Payload builder tests ───────────────────────────────────────

    #[test]
    fn opening_torpedo_payload_builder_routes_damage_by_attacker_side() {
        let mut payload = BattleOpeningAttack::blank(2);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: 1,
                defender_index: 0,
                damage: 21,
                shield: false,
            },
        );
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: 0,
                defender_index: 1,
                damage: 34,
                shield: false,
            },
        );
        let opening = payload;

        assert_eq!(opening.api_frai_list_items[1], Some(vec![0]));
        assert_eq!(opening.api_fydam_list_items[1], Some(vec![DamageCell::Plain(21)]));
        assert_eq!(opening.api_eydam_list_items[1], None);
        assert_eq!(opening.api_edam[0], 21);
        assert_eq!(opening.api_erai_list_items[0], Some(vec![1]));
        assert_eq!(opening.api_eydam_list_items[0], Some(vec![DamageCell::Plain(34)]));
        assert_eq!(opening.api_fydam_list_items[0], None);
        assert_eq!(opening.api_fdam[1], 34);
    }

    #[test]
    fn raigeki_payload_builder_routes_damage_by_attacker_side() {
        let mut payload = BattleRaigeki::blank(2);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: 1,
                defender_index: 0,
                damage: 21,
                shield: false,
            },
        );
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: 0,
                defender_index: 1,
                damage: 34,
                shield: false,
            },
        );
        let raigeki = payload;

        assert_eq!(raigeki.api_frai[1], 0);
        assert_eq!(raigeki.api_fydam[1], DamageCell::Plain(21));
        assert_eq!(raigeki.api_eydam[1], DamageCell::Plain(0));
        assert_eq!(raigeki.api_edam[0], 21);
        assert_eq!(raigeki.api_erai[0], 1);
        assert_eq!(raigeki.api_eydam[0], DamageCell::Plain(34));
        assert_eq!(raigeki.api_fydam[0], DamageCell::Plain(0));
        assert_eq!(raigeki.api_fdam[1], 34);
    }
}
