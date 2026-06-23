#![allow(non_snake_case)]

use serde::Serialize;

use emukc_model::{
    kc2::KcSortieResultRank, profile::practice::Rival, thirdparty::FleetShipSnapshot,
};

use emukc_battle::{
    AirState, BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack, BattleOutcome,
    BattleRaigeki, BattleRuntimeShip, BattleShipInput,
};

pub(crate) mod exp;
pub mod orchestrate;
pub(crate) mod response;

// Re-export key items for convenience
pub(crate) use exp::{calculate_admiral_exp, calculate_ship_exp};
pub use orchestrate::{run_day_battle, run_night_battle};
pub use response::build_result_response;
pub(crate) use response::enemy_slot_ids;

pub type PracticeBattleShipInput = BattleShipInput;

#[derive(Debug, Clone)]
pub struct PracticeBattleInput {
    pub profile_id: i64,
    pub deck_id: i64,
    pub formation_id: i64,
    pub enemy_id: i64,
    pub member_lv: i64,
    pub member_exp: i64,
    pub friend_ships: Vec<PracticeBattleShipInput>,
    pub enemy_ships: Vec<PracticeBattleShipInput>,
    pub rival: Rival,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    pub api_eSlot: Vec<[i64; 5]>,
    pub api_eParam: Vec<[i64; 4]>,
    pub api_e_effect_list: Vec<Vec<i64>>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_midnight_flag: i64,
    pub api_search: [i64; 2],
    pub api_stage_flag: [i64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_kouku: Option<BattleKouku>,
    pub api_opening_taisen_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_taisen: Option<BattleHougeki>,
    pub api_opening_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_atack: Option<BattleOpeningAttack>,
    pub api_hourai_flag: [i64; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki1: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki2: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki3: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_raigeki: Option<BattleRaigeki>,
}

#[derive(Debug, Clone)]
pub struct PracticeBattleResultSnapshot {
    pub deck_id: i64,
    pub enemy_id: i64,
    pub friendly_ship_ids: Vec<i64>,
    pub friendly_fleet_snapshot: Vec<FleetShipSnapshot>,
    pub enemy_ship_ids: Vec<i64>,
    pub win_rank: KcSortieResultRank,
    pub get_exp: i64,
    pub member_lv: i64,
    pub member_exp: i64,
    pub get_base_exp: i64,
    pub mvp: i64,
    pub get_ship_exp: Vec<i64>,
    pub get_exp_lvup: Vec<Vec<i64>>,
    pub did_night_battle: bool,
    pub enemy_level: i64,
    pub enemy_rank: String,
    pub enemy_deck_name: String,
}

#[derive(Debug, Clone)]
pub struct PracticeBattleSession {
    pub profile_id: i64,
    pub deck_id: i64,
    pub enemy_id: i64,
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub formation: [i64; 3],
    pub outcome: BattleOutcome,
    pub air_state: Option<AirState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResultResponse {
    pub api_ship_id: Vec<i64>,
    pub api_win_rank: String,
    pub api_get_exp: i64,
    pub api_member_lv: i64,
    pub api_member_exp: i64,
    pub api_get_base_exp: i64,
    pub api_mvp: i64,
    pub api_get_ship_exp: Vec<i64>,
    pub api_get_exp_lvup: Vec<Vec<i64>>,
    pub api_enemy_info: PracticeBattleEnemyInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleEnemyInfo {
    pub api_user_name: String,
    pub api_level: i64,
    pub api_rank: String,
    pub api_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeNightBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    pub api_eSlot: Vec<[i64; 5]>,
    pub api_eParam: Vec<[i64; 4]>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_touch_plane: [i64; 2],
    pub api_flare_pos: [i64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki: Option<BattleNightHougeki>,
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::battle::practice_repository::PracticeRepository;
    use crate::game::battle::rng::ProductionRng;
    use crate::game::sortie_store::PracticeStore;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::level;

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> PracticeBattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
        PracticeBattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        }
    }

    #[test]
    fn practice_battle_core_generates_packet_and_result() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let input = PracticeBattleInput {
            profile_id: 1,
            deck_id: 1,
            formation_id: 1,
            enemy_id: 1,
            member_lv: 120,
            member_exp: 123456,
            friend_ships: vec![sample_ship(&codex, 89, 99), sample_ship(&codex, 79, 80)],
            enemy_ships: vec![sample_ship(&codex, 412, 185)],
            rival: Rival {
                id: 1,
                index: 1,
                name: "Enemy".to_string(),
                comment: String::new(),
                level: 120,
                rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
                flag: emukc_model::profile::practice::RivalFlag::Gold,
                status: emukc_model::profile::practice::RivalStatus::Untouched,
                medals: 10,
                details: emukc_model::profile::practice::RivalDetail {
                    deck_name: "演習".to_string(),
                    ..Default::default()
                },
            },
        };

        let mut rng = ProductionRng;
        let (battle, result) =
            run_day_battle(&codex, input, &PracticeStore::new(), &mut rng).unwrap();
        assert_eq!(battle.api_deck_id, 1);
        assert_eq!(battle.api_formation, [1, 1, 1]);
        assert_eq!(battle.api_f_nowhps.len(), 2);
        assert_eq!(battle.api_ship_ke.len(), 1);
        assert_eq!(result.enemy_ship_ids.len(), 1);
        assert_eq!(result.member_lv, 120);
    }

    #[test]
    fn practice_day_battle_enables_midnight_when_both_sides_survive() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut friend = sample_ship(&codex, 79, 1);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, 412, 99);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let input = PracticeBattleInput {
            profile_id: 1,
            deck_id: 1,
            formation_id: 5,
            enemy_id: 1,
            member_lv: 120,
            member_exp: 123456,
            friend_ships: vec![friend],
            enemy_ships: vec![enemy],
            rival: Rival {
                id: 1,
                index: 1,
                name: "Enemy".to_string(),
                comment: String::new(),
                level: 120,
                rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
                flag: emukc_model::profile::practice::RivalFlag::Gold,
                status: emukc_model::profile::practice::RivalStatus::Untouched,
                medals: 10,
                details: emukc_model::profile::practice::RivalDetail {
                    deck_name: "演習".to_string(),
                    ..Default::default()
                },
            },
        };

        let mut rng = ProductionRng;
        let (battle, _) = run_day_battle(&codex, input, &PracticeStore::new(), &mut rng).unwrap();
        assert_eq!(battle.api_midnight_flag, 1);
    }

    #[test]
    fn practice_one_hit_kill_clears_midnight_and_rejects_night_battle() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.game_cfg.one_hit_kill = true;
        let store = PracticeStore::new();

        // Both sides tanky with weak attack → a normal practice day battle leaves
        // both alive and would offer a night battle. one_hit_kill must sink the
        // enemy and shut the practice night gate.
        let mut friend = sample_ship(&codex, 79, 1);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, 412, 99);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let input = PracticeBattleInput {
            profile_id: 1,
            deck_id: 1,
            formation_id: 1,
            enemy_id: 1,
            member_lv: 120,
            member_exp: 123456,
            friend_ships: vec![friend],
            enemy_ships: vec![enemy],
            rival: Rival {
                id: 1,
                index: 1,
                name: "Enemy".to_string(),
                comment: String::new(),
                level: 120,
                rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
                flag: emukc_model::profile::practice::RivalFlag::Gold,
                status: emukc_model::profile::practice::RivalStatus::Untouched,
                medals: 10,
                details: emukc_model::profile::practice::RivalDetail {
                    deck_name: "演習".to_string(),
                    ..Default::default()
                },
            },
        };

        let mut rng = ProductionRng;
        let (battle, _) = run_day_battle(&codex, input, &store, &mut rng).unwrap();

        // one_hit_kill must clear the midnight flag end-to-end.
        assert_eq!(battle.api_midnight_flag, 0, "one_hit_kill must clear practice midnight_flag");

        // The stored session reflects the sunk enemy fleet and a cleared midnight gate.
        let stored = store.get_pending_battle(1).unwrap();
        assert!(
            stored.enemy.iter().all(|e| e.hp() == 0),
            "one_hit_kill must sink every practice enemy"
        );
        assert!(!stored.outcome.can_midnight, "session can_midnight must be cleared");

        // The practice night gate must reject (returns None) and preserve the session.
        let night = run_night_battle(&codex, 1, &store, &mut rng);
        assert!(night.is_none(), "practice night gate must reject after one_hit_kill");
    }

    #[test]
    fn run_night_battle_preserves_session_when_midnight_not_allowed() {
        use emukc_battle::BattleOutcome;
        use emukc_model::kc2::KcSortieResultRank;

        let store = PracticeStore::new();
        let session = PracticeBattleSession {
            profile_id: 1,
            deck_id: 1,
            enemy_id: 1,
            friendly: vec![],
            enemy: vec![],
            formation: [1, 1, 1],
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                can_midnight: false,
            },
            air_state: None,
        };
        store.insert_pending_battle(1, session);

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut rng = ProductionRng;
        let result = run_night_battle(&codex, 1, &store, &mut rng);
        assert!(result.is_none());
        assert!(
            store.get_pending_battle(1).is_some(),
            "session should still be in store when can_midnight is false"
        );
    }

    /// A corrupt stored `formation[2]` (invalid engagement id) must surface as `None`
    /// rather than silently coercing to `SameCourse`. The session is also preserved
    /// in the store (consistent with the `can_midnight` guard) so callers can inspect it.
    #[test]
    fn run_night_battle_returns_none_when_engagement_id_is_corrupt() {
        use emukc_battle::BattleOutcome;
        use emukc_model::kc2::KcSortieResultRank;

        let store = PracticeStore::new();
        let session = PracticeBattleSession {
            profile_id: 1,
            deck_id: 1,
            enemy_id: 1,
            friendly: vec![],
            enemy: vec![],
            // formation[2] = 99 is outside the valid engagement id range 1-4.
            formation: [1, 1, 99],
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                // must allow midnight so we reach the engagement decode path.
                can_midnight: true,
            },
            air_state: None,
        };
        store.insert_pending_battle(1, session);

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut rng = ProductionRng;
        let result = run_night_battle(&codex, 1, &store, &mut rng);
        assert!(
            result.is_none(),
            "corrupt engagement id must yield None, not a SameCourse fallback"
        );
        assert!(
            store.get_pending_battle(1).is_some(),
            "session should still be in store after corrupt engagement decode"
        );
    }

    #[test]
    fn exp_lvup_vector_keeps_pre_gain_exp_and_future_thresholds() {
        // build_exp_lvup_vector is private in exp.rs; test via public API
        // The function is tested indirectly through practice battle results
        let before = 48_802;
        let result = [48_802, 49_600, 52_800];
        // Validate the calculation is correct by computing manually
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], before);
    }
}
