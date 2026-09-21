//! Battle API response wire structs and their builders.
//!
//! Sortie and practice emit the same two wire shapes, so both the structs and
//! the packet-to-wire translation live here once.

use serde::Serialize;

use emukc_battle::{
    BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleShipInput, NightBattlePacket,
};
use emukc_model::kc2::{KcApiShip, KcApiSlotItem};

/// Day battle API response.
#[expect(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct DayBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    /// 第2艦隊's half of the friendly arrays, absent for a single fleet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_f_nowhps_combined: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_f_maxhps_combined: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_fParam_combined: Option<Vec<[i64; 4]>>,
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

/// Night battle API response.
#[expect(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct NightBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    /// 第2艦隊's half of the friendly arrays, absent for a single fleet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_f_nowhps_combined: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_f_maxhps_combined: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_fParam_combined: Option<Vec<[i64; 4]>>,
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

impl DayBattleResponse {
    /// Attach 第2艦隊 to a response whose friendly arrays hold 第1艦隊.
    ///
    /// The client reads the two decks from separate arrays, so they never share
    /// one — unlike the shelling payloads, where both decks live in one 0..=11
    /// index space that `emukc_battle` has already translated into.
    #[must_use]
    pub fn with_escort_deck(mut self, escort: &[BattleShipInput]) -> Self {
        self.api_f_nowhps_combined = Some(escort.iter().map(|ship| ship.ship.api_nowhp).collect());
        self.api_f_maxhps_combined = Some(escort.iter().map(|ship| ship.ship.api_maxhp).collect());
        self.api_fParam_combined =
            Some(escort.iter().map(|ship| ship_params(&ship.ship)).collect());
        self
    }
}

impl NightBattleResponse {
    /// Attach 第1艦隊 to a response whose friendly arrays hold 第2艦隊.
    ///
    /// A combined night battle is fought by 第2艦隊 alone, so the simulation —
    /// and therefore this response — starts out carrying the escort deck in the
    /// plain `api_f_*` arrays. This moves them to the `_combined` ones and fills
    /// the plain arrays from 第1艦隊, which sat the battle out and is reported at
    /// the HP it entered the night with.
    #[must_use]
    pub fn with_main_deck(mut self, main: &[BattleRuntimeShip]) -> Self {
        self.api_f_nowhps_combined = Some(std::mem::take(&mut self.api_f_nowhps));
        self.api_f_maxhps_combined = Some(std::mem::take(&mut self.api_f_maxhps));
        self.api_fParam_combined = Some(std::mem::take(&mut self.api_fParam));
        self.api_f_nowhps = main.iter().map(|ship| ship.hp().max(0)).collect();
        self.api_f_maxhps = main.iter().map(|ship| ship.ship.api_maxhp).collect();
        self.api_fParam = main.iter().map(|ship| ship_params(&ship.ship)).collect();
        self
    }
}

/// Map a ship's slot items to the 5-element array expected by the API.
///
/// Prefers actual slot contents over slot-item IDs when slots are populated.
pub(crate) fn enemy_slot_ids(ship: &KcApiShip, slot_items: &[KcApiSlotItem]) -> [i64; 5] {
    if ship.api_slot.iter().any(|slot| *slot > 0) {
        let mut slots = [-1; 5];
        for (idx, slot) in ship.api_slot.iter().take(5).enumerate() {
            if *slot > 0 {
                slots[idx] = *slot;
            }
        }
        return slots;
    }
    let mut slots = [-1; 5];
    for (idx, slot_item) in slot_items.iter().take(5).enumerate() {
        slots[idx] = slot_item.api_slotitem_id;
    }
    slots
}

/// The `api_fParam` / `api_eParam` quadruple: firepower, torpedo, AA, armor.
fn ship_params(ship: &KcApiShip) -> [i64; 4] {
    [ship.api_karyoku[0], ship.api_raisou[0], ship.api_taiku[0], ship.api_soukou[0]]
}

/// Build a day-battle API response.
///
/// HP arrays come from the ships as they entered the node, not from the packet.
pub fn build_day_response(
    deck_id: i64,
    friendly: &[BattleShipInput],
    enemy: &[BattleShipInput],
    packet: BattlePacket,
) -> DayBattleResponse {
    DayBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: friendly.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_f_maxhps: friendly.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_fParam: friendly.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_f_nowhps_combined: None,
        api_f_maxhps_combined: None,
        api_fParam_combined: None,
        api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_e_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_eSlot: enemy.iter().map(|ship| enemy_slot_ids(&ship.ship, &ship.slot_items)).collect(),
        api_eParam: enemy.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_e_effect_list: enemy
            .iter()
            .map(|ship| {
                if ship.effect_list.is_empty() {
                    vec![0]
                } else {
                    ship.effect_list.clone()
                }
            })
            .collect(),
        api_smoke_type: packet.smoke_type,
        api_balloon_cell: packet.balloon_cell,
        api_atoll_cell: packet.atoll_cell,
        api_midnight_flag: packet.midnight_flag,
        api_search: packet.search,
        api_stage_flag: packet.stage_flag,
        api_kouku: packet.kouku,
        api_opening_taisen_flag: packet.opening_taisen_flag,
        api_opening_taisen: packet.opening_taisen,
        api_opening_flag: packet.opening_flag,
        api_opening_atack: packet.opening_attack,
        api_hourai_flag: packet.hourai_flag,
        api_hougeki1: packet.hougeki1,
        api_hougeki2: packet.hougeki2,
        api_hougeki3: packet.hougeki3,
        api_raigeki: packet.raigeki,
    }
}

/// Build a night-battle API response.
///
/// HP arrays come from the packet, which already reflects the night phase.
pub fn build_night_response(
    deck_id: i64,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
    packet: NightBattlePacket,
) -> NightBattleResponse {
    NightBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps,
        api_f_maxhps: packet.friendly_maxhps,
        api_fParam: friendly.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_f_nowhps_combined: None,
        api_f_maxhps_combined: None,
        api_fParam_combined: None,
        api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: packet.enemy_nowhps,
        api_e_maxhps: packet.enemy_maxhps,
        api_eSlot: enemy.iter().map(|ship| enemy_slot_ids(&ship.ship, &ship.slot_items)).collect(),
        api_eParam: enemy.iter().map(|ship| ship_params(&ship.ship)).collect(),
        api_smoke_type: 0,
        api_balloon_cell: 0,
        api_atoll_cell: 0,
        api_touch_plane: packet.touch_plane,
        api_flare_pos: packet.flare_pos,
        api_hougeki: packet.hougeki,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ship(nowhp: i64, maxhp: i64) -> KcApiShip {
        KcApiShip {
            api_id: 1,
            api_sortno: 1,
            api_ship_id: 1,
            api_lv: 1,
            api_exp: [0, 0, 0],
            api_nowhp: nowhp,
            api_maxhp: maxhp,
            api_soku: 10,
            api_leng: 1,
            api_slot: [-1; 5],
            api_onslot: [0; 5],
            api_slot_ex: 0,
            api_onslot_max: None,
            api_kyouka: [0; 7],
            api_backs: 1,
            api_fuel: 0,
            api_bull: 0,
            api_slotnum: 4,
            api_ndock_time: 0,
            api_ndock_item: [0; 2],
            api_srate: 0,
            api_cond: 49,
            api_karyoku: [0; 2],
            api_raisou: [0; 2],
            api_taiku: [0; 2],
            api_soukou: [0; 2],
            api_kaihi: [0; 2],
            api_taisen: [0; 2],
            api_sakuteki: [0; 2],
            api_lucky: [0; 2],
            api_locked: 0,
            api_locked_equip: 0,
            api_sally_area: 0,
            api_sp_effect_items: None,
        }
    }

    fn test_slot_item(slotitem_id: i64) -> KcApiSlotItem {
        KcApiSlotItem {
            api_id: slotitem_id,
            api_slotitem_id: slotitem_id,
            api_locked: 0,
            api_level: 0,
            api_alv: None,
        }
    }

    fn test_input(
        ship: KcApiShip,
        slot_items: Vec<KcApiSlotItem>,
        effect_list: Vec<i64>,
    ) -> BattleShipInput {
        BattleShipInput {
            ship,
            slot_items,
            effect_list,
            married: false,
        }
    }

    fn test_packet() -> BattlePacket {
        BattlePacket {
            formation: [1, 1, 1],
            friendly_nowhps: vec![],
            enemy_nowhps: vec![],
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: 0,
            search: [1, 1],
            stage_flag: [0, 0, 0],
            kouku: None,
            opening_taisen_flag: 0,
            opening_taisen: None,
            opening_flag: 0,
            opening_attack: None,
            hourai_flag: [0, 0, 0, 0],
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
        }
    }

    #[test]
    fn day_response_replaces_an_empty_enemy_effect_list_with_zero() {
        let friendly = vec![test_input(test_ship(10, 10), vec![], vec![0])];
        let enemy = vec![
            test_input(test_ship(20, 20), vec![], vec![]),
            test_input(test_ship(20, 20), vec![], vec![7, 8]),
        ];

        let resp = build_day_response(1, &friendly, &enemy, test_packet());

        assert_eq!(resp.api_e_effect_list, vec![vec![0], vec![7, 8]]);
    }

    #[test]
    fn day_response_falls_back_to_slot_items_when_api_slot_is_empty() {
        let friendly = vec![test_input(test_ship(10, 10), vec![], vec![0])];
        let mut equipped = test_ship(20, 20);
        equipped.api_slot = [101, 0, -1, -1, -1];
        let enemy = vec![
            test_input(equipped, vec![test_slot_item(999)], vec![0]),
            test_input(test_ship(20, 20), vec![test_slot_item(525)], vec![0]),
        ];

        let resp = build_day_response(1, &friendly, &enemy, test_packet());

        assert_eq!(resp.api_eSlot, vec![[101, -1, -1, -1, -1], [525, -1, -1, -1, -1]]);
    }

    #[test]
    fn day_response_reports_entry_hp_not_packet_hp() {
        let friendly = vec![test_input(test_ship(31, 40), vec![], vec![0])];
        let enemy = vec![test_input(test_ship(12, 20), vec![], vec![0])];
        let mut packet = test_packet();
        packet.friendly_nowhps = vec![3];
        packet.enemy_nowhps = vec![0];

        let resp = build_day_response(1, &friendly, &enemy, packet);

        assert_eq!(resp.api_f_nowhps, vec![31]);
        assert_eq!(resp.api_f_maxhps, vec![40]);
        assert_eq!(resp.api_e_nowhps, vec![12]);
        assert_eq!(resp.api_e_maxhps, vec![20]);
    }
}

/// Response-shape tests for a friendly combined fleet against a single enemy
/// fleet: which deck each shelling round carries, and how the friendly arrays
/// split. The two endpoints are mirror images
/// (`docs/battle/combined-fleet-reference.md` §Which shelling round carries
/// which deck), so neither test covers the other.
#[cfg(test)]
mod combined_tests {
    use emukc_battle::{
        BattleContext, BattleHougeki, BattleRng, BattleShipInput, BattleType, CombinedSetup,
        CombinedType, ESCORT_INDEX_OFFSET, EngagementType, execute_day,
    };
    use emukc_crypto::rng::GameRng;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::{level, types::KcShipType};

    use super::{DayBattleResponse, build_day_response};

    const SEED: u64 = 0x0CB1_9E7D;

    /// `emukc_battle`'s own `SeededRng` is `#[cfg(test)]` and so invisible from
    /// this crate. Same `GameRng` backend and seeding, so it draws the same
    /// sequence.
    struct SeededRng {
        inner: GameRng,
    }

    impl SeededRng {
        fn new(seed: u64) -> Self {
            Self {
                inner: GameRng::seeded(seed),
            }
        }
    }

    impl BattleRng for SeededRng {
        fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
            self.inner.f64_range(min, max)
        }

        fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
            self.inner.i64(min..max)
        }
    }

    fn load_codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex")
            .expect("load codex from ../../.data/codex (run `cargo run -- bootstrap` first)")
    }

    fn mst_id_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
        codex
            .manifest
            .api_mst_ship
            .iter()
            .find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
            .map(|mst| mst.api_id)
            .expect("codex must hold a ship of this type")
    }

    fn ship(codex: &Codex, mst_id: i64) -> BattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(99);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = 99;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        }
    }

    /// An armour-plated, high-HP ship. Nobody sinks, so every shelling round
    /// runs and the assertions below are about ordering rather than luck.
    fn armoured(codex: &Codex, mst_id: i64) -> BattleShipInput {
        let mut input = ship(codex, mst_id);
        input.ship.api_soukou[0] = 500;
        input.ship.api_maxhp = 900;
        input.ship.api_nowhp = 900;
        input
    }

    /// 第1艦隊 of two — deliberately shorter than six, so the gap between deck
    /// 1's last ship and packet index 6 has to be there.
    fn main_deck(codex: &Codex) -> Vec<BattleShipInput> {
        vec![
            armoured(codex, mst_id_by_type(codex, KcShipType::BB)),
            armoured(codex, mst_id_by_type(codex, KcShipType::DD)),
        ]
    }

    fn escort_deck(codex: &Codex) -> Vec<BattleShipInput> {
        let dd = mst_id_by_type(codex, KcShipType::DD);
        vec![armoured(codex, dd), armoured(codex, dd), armoured(codex, dd)]
    }

    fn enemy_fleet(codex: &Codex) -> Vec<BattleShipInput> {
        let ca = mst_id_by_type(codex, KcShipType::CA);
        vec![armoured(codex, ca), armoured(codex, ca), armoured(codex, ca)]
    }

    fn combined_response(codex: &Codex, combined_type: CombinedType) -> DayBattleResponse {
        let main = main_deck(codex);
        let escort = escort_deck(codex);
        let enemy = enemy_fleet(codex);
        let context = BattleContext {
            battle_type: BattleType::Normal,
            is_sortie: true,
            // 第一警戒航行序列 — the one formation with no escort-size floor.
            friendly_formation_id: 11,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: main.clone(),
            enemy_ships: enemy.clone(),
            combined: Some(CombinedSetup {
                combined_type,
                escort_ships: escort.clone(),
            }),
        };
        let mut rng = SeededRng::new(SEED);
        let simulation = execute_day(codex, context, &mut rng);
        build_day_response(1, &main, &enemy, simulation.packet).with_escort_deck(&escort)
    }

    /// Packet indices of the friendly attackers in one shelling round.
    fn friendly_attackers(round: Option<&BattleHougeki>) -> Vec<i64> {
        round.map_or_else(Vec::new, |hougeki| {
            hougeki
                .api_at_eflag
                .iter()
                .zip(hougeki.api_at_list.iter())
                .filter(|(eflag, _)| **eflag == 0)
                .map(|(_, attacker)| *attacker)
                .collect()
        })
    }

    fn assert_all_main_deck(round: Option<&BattleHougeki>, label: &str) {
        let attackers = friendly_attackers(round);
        assert!(!attackers.is_empty(), "{label} must hold at least one friendly attack");
        for index in attackers {
            assert!(
                index < ESCORT_INDEX_OFFSET as i64,
                "{label} belongs to 第1艦隊, but attacker {index} is in 第2艦隊's index range"
            );
        }
    }

    fn assert_all_escort_deck(round: Option<&BattleHougeki>, label: &str) {
        let attackers = friendly_attackers(round);
        assert!(!attackers.is_empty(), "{label} must hold at least one friendly attack");
        for index in attackers {
            assert!(
                index >= ESCORT_INDEX_OFFSET as i64,
                "{label} belongs to 第2艦隊, but attacker {index} is in 第1艦隊's index range"
            );
        }
    }

    /// `api_req_combined_battle/battle` (空母機動 / 輸送護衛,
    /// `docs/apilist.txt:3008`): 第2艦隊 takes `api_hougeki1` and 第1艦隊's two
    /// rounds land in slots 2 and 3, with 雷撃 between them.
    #[test]
    fn carrier_task_force_response_puts_the_escort_deck_in_hougeki1() {
        let codex = load_codex();
        let resp = combined_response(&codex, CombinedType::CarrierTaskForce);

        assert_all_escort_deck(resp.api_hougeki1.as_ref(), "api_hougeki1");
        assert_all_main_deck(resp.api_hougeki2.as_ref(), "api_hougeki2");
        assert_all_main_deck(resp.api_hougeki3.as_ref(), "api_hougeki3");
    }

    /// `api_req_combined_battle/battle_water` (水上打撃,
    /// `docs/apilist.txt:3164`) is the mirror image: 第1艦隊 takes slots 1 and 2
    /// and 第2艦隊 drops to slot 3, with 雷撃 last.
    #[test]
    fn surface_task_force_response_puts_the_main_deck_in_hougeki1_and_2() {
        let codex = load_codex();
        let resp = combined_response(&codex, CombinedType::SurfaceTaskForce);

        assert_all_main_deck(resp.api_hougeki1.as_ref(), "api_hougeki1");
        assert_all_main_deck(resp.api_hougeki2.as_ref(), "api_hougeki2");
        assert_all_escort_deck(resp.api_hougeki3.as_ref(), "api_hougeki3");
    }

    /// Both decks' own arrays are present and sized to their own deck; a
    /// combined response never merges them.
    #[test]
    fn friendly_arrays_split_by_deck() {
        let codex = load_codex();
        let resp = combined_response(&codex, CombinedType::CarrierTaskForce);

        assert_eq!(resp.api_f_nowhps.len(), 2, "api_f_nowhps is 第1艦隊 alone");
        assert_eq!(resp.api_f_maxhps.len(), 2);
        assert_eq!(resp.api_fParam.len(), 2);
        assert_eq!(resp.api_f_nowhps_combined.as_ref().map(Vec::len), Some(3));
        assert_eq!(resp.api_f_maxhps_combined.as_ref().map(Vec::len), Some(3));
        assert_eq!(resp.api_fParam_combined.as_ref().map(Vec::len), Some(3));

        let json = serde_json::to_value(&resp).unwrap();
        for key in ["api_f_nowhps_combined", "api_f_maxhps_combined", "api_fParam_combined"] {
            assert!(json.get(key).is_some(), "{key} must be on the wire");
        }
        // 第2艦隊 fought, so the airstrike's stage 3 is split too.
        if let Some(kouku) = json.get("api_kouku") {
            assert!(
                kouku.get("api_stage3_combined").is_some(),
                "api_stage3 must split when the fleet is combined"
            );
        }
    }

    /// The same builder without `with_escort_deck` emits no combined key at
    /// all — the single-fleet wire shape is unchanged.
    #[test]
    fn single_fleet_response_carries_no_combined_keys() {
        let codex = load_codex();
        let main = main_deck(&codex);
        let enemy = enemy_fleet(&codex);
        let context = BattleContext::head_on(BattleType::Normal, true, main.clone(), enemy.clone());
        let mut rng = SeededRng::new(SEED);
        let simulation = execute_day(&codex, context, &mut rng);
        let resp = build_day_response(1, &main, &enemy, simulation.packet);

        let json = serde_json::to_value(&resp).unwrap();
        for key in ["api_f_nowhps_combined", "api_f_maxhps_combined", "api_fParam_combined"] {
            assert!(json.get(key).is_none(), "{key} must be absent for a single fleet");
        }
        if let Some(kouku) = json.get("api_kouku") {
            assert!(kouku.get("api_stage3_combined").is_none());
        }
    }
}
