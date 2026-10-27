//! Importing a live capture into a fresh profile (plan 2026-10-27-001, U1–U5).
//!
//! The fixture is synthetic: two ships, three pieces of equipment (one with
//! proficiency, one in the reinforcement slot), two fleets, a ship under repair and a
//! fleet on an expedition. No value comes from a real account.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        model::{
            kc2::live::LiveSnapshot,
            profile::{kdock::ConstructionDockStatus, ndock::RepairDockStatus},
        },
        prelude::*,
    };
    use serde_json::{Value, json};

    /// Far enough ahead that no timer runs out while a test reads it back.
    const LATER_MS: i64 = 4_102_444_800_000; // 2100-01-01

    fn ship(id: i64, mst: i64, lv: i64, slot: [i64; 5], slot_ex: i64) -> Value {
        json!({
            "api_id": id, "api_sortno": 1, "api_ship_id": mst, "api_lv": lv,
            "api_exp": [0, 100, 0], "api_nowhp": 10, "api_maxhp": 16, "api_soku": 10,
            "api_leng": 1, "api_slot": slot, "api_onslot": [0, 0, 0, 0, 0],
            "api_slot_ex": slot_ex, "api_kyouka": [0, 0, 0, 0, 0, 0, 0], "api_backs": 3,
            "api_fuel": 15, "api_bull": 20, "api_slotnum": 3, "api_ndock_time": 0,
            "api_ndock_item": [0, 0], "api_srate": 0, "api_cond": 49,
            "api_karyoku": [10, 30], "api_raisou": [20, 60], "api_taiku": [10, 40],
            "api_soukou": [5, 20], "api_kaihi": [40, 80], "api_taisen": [20, 50],
            "api_sakuteki": [5, 20], "api_lucky": [10, 50], "api_locked": 1,
            "api_locked_equip": 0,
        })
    }

    /// `api_port/port` and `api_get_member/require_info`, as `api_data`.
    fn fixture() -> (Value, Value) {
        // Ship 50 is listed before ship 7 and the equipment out of order, so a test
        // fails if the import trusts the capture's order instead of sorting by id.
        let mut houshou = ship(50, 89, 30, [300, -1, -1, -1, -1], -1);
        houshou["api_slotnum"] = json!(2);
        // A white sash (useitem 99) on 鳳翔: the 10-27 capture has none.
        houshou["api_sp_effect_items"] = json!([{ "api_kind": 2, "api_kaih": 2, "api_souk": 1 }]);
        let amatsukaze = ship(7, 951, 120, [12, -1, -1, -1, -1], 77);
        let materials: Vec<Value> = (1..=8)
            .map(|id| json!({ "api_member_id": 12345, "api_id": id, "api_value": 1000 * id }))
            .collect();
        let furniture = [2, 39, 73, 103, 134, 165, 500].map(|id| {
            json!({ "api_id": id, "api_furniture_type": 0, "api_furniture_no": 0, "api_furniture_id": id })
        });
        let port = json!({
            "api_basic": {
                "api_member_id": "12345", "api_nickname": "fixture", "api_nickname_id": "1",
                "api_active_flag": 1, "api_starttime": 0, "api_level": 42, "api_rank": 5,
                "api_experience": 123_456, "api_fleetname": null, "api_comment": "hi",
                "api_comment_id": "1", "api_max_chara": 120, "api_max_slotitem": 520,
                "api_max_kagu": 0, "api_playtime": 0, "api_tutorial": 0,
                "api_furniture": [2, 39, 73, 103, 134, 165], "api_count_deck": 2,
                "api_count_kdock": 2, "api_count_ndock": 2, "api_fcoin": 4321,
                "api_st_win": 10, "api_st_lose": 2, "api_ms_count": 30, "api_ms_success": 29,
                "api_pt_win": 5, "api_pt_lose": 1, "api_pt_challenged": 3,
                "api_pt_challenged_win": 2, "api_firstflag": 1, "api_tutorial_progress": 100,
                "api_pvp": [0, 0], "api_medals": 1, "api_large_dock": 1,
            },
            "api_ship": [houshou, amatsukaze],
            "api_deck_port": [
                { "api_member_id": 12345, "api_id": 1, "api_name": "第1艦隊", "api_name_id": "",
                  "api_mission": [0, 0, 0, 0], "api_flagship": "0", "api_ship": [50, -1, -1, -1, -1, -1] },
                { "api_member_id": 12345, "api_id": 2, "api_name": "遠征隊", "api_name_id": "",
                  "api_mission": [1, 2, LATER_MS, 0], "api_flagship": "0", "api_ship": [7, -1, -1, -1, -1, -1] },
            ],
            "api_ndock": [
                { "api_member_id": 12345, "api_id": 1, "api_state": 1, "api_ship_id": 50,
                  "api_complete_time": LATER_MS, "api_complete_time_str": "", "api_item1": 10,
                  "api_item2": 0, "api_item3": 20, "api_item4": 0 },
                { "api_member_id": 12345, "api_id": 2, "api_state": 0, "api_ship_id": 0,
                  "api_complete_time": 0, "api_complete_time_str": "0", "api_item1": 0,
                  "api_item2": 0, "api_item3": 0, "api_item4": 0 },
            ],
            "api_material": materials,
            "api_p_bgm_id": 140,
            "api_parallel_quest_count": 7,
        });
        let require_info = json!({
            "api_slot_item": [
                { "api_id": 300, "api_slotitem_id": 19, "api_locked": 0, "api_level": 0, "api_alv": 7 },
                { "api_id": 12, "api_slotitem_id": 2, "api_locked": 1, "api_level": 6 },
                { "api_id": 77, "api_slotitem_id": 43, "api_locked": 0, "api_level": 0 },
            ],
            "api_kdock": [
                { "api_id": 1, "api_state": 2, "api_created_ship_id": 951, "api_complete_time": LATER_MS,
                  "api_complete_time_str": "", "api_item1": 1500, "api_item2": 1500, "api_item3": 2000,
                  "api_item4": 1000, "api_item5": 20 },
                { "api_id": 2, "api_state": -1, "api_created_ship_id": 0, "api_complete_time": 0,
                  "api_complete_time_str": "", "api_item1": 0, "api_item2": 0, "api_item3": 0,
                  "api_item4": 0, "api_item5": 0 },
            ],
            "api_useitem": [{ "api_id": 10, "api_count": 3 }],
            "api_furniture": furniture,
            "api_extra_supply": [1, 0],
            "api_oss_setting": { "api_language_type": 1, "api_oss_items": [0, 0, 0, 0, 1, 0, 0, 0] },
            "api_skin_id": 311,
            "api_position_id": 1,
        });
        (port, require_info)
    }

    fn snapshot() -> LiveSnapshot {
        let (port, require_info) = fixture();
        LiveSnapshot::from_api_data(&port, &require_info).expect("the fixture parses")
    }

    async fn new_profile(context: &crate::TestContext, name: &str) -> i64 {
        let account = context.sign_up(name, "1234567").await.unwrap();
        context.new_profile(&account.access_token.token, name).await.unwrap().profile.id
    }

    #[test]
    fn a_missing_field_names_its_endpoint() {
        let (mut port, require_info) = fixture();
        port.as_object_mut().unwrap().remove("api_ship");
        let error = LiveSnapshot::from_api_data(&port, &require_info).unwrap_err().to_string();
        assert_eq!(error, "api_port/port: `api_ship` is missing");
    }

    #[tokio::test]
    async fn the_profile_and_the_simple_domains_take_the_capture() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "import-simple").await;
        context.import_account(pid, &snapshot()).await.unwrap();

        let (profile, basic) = context.get_user_basic(pid).await.unwrap();
        assert_eq!(
            (profile.hq_level, profile.max_ship_capacity, profile.max_equipment_capacity),
            (42, 120, 520)
        );
        assert_eq!((profile.max_quests, profile.tutorial_progress), (7, 100));
        assert!(profile.intro_completed && profile.large_dock_unlocked);
        assert!(profile.extra_supply_expedition && !profile.extra_supply_sortie);
        assert_eq!(basic.api_fcoin, 4321);

        let materials = context.get_materials(pid).await.unwrap();
        assert_eq!([materials.fuel, materials.bucket, materials.screw], [1000, 6000, 8000]);
        let use_items = context.get_use_items(pid).await.unwrap();
        assert!(use_items.iter().any(|item| item.api_id == 10 && item.api_count == 3));

        let furniture: Vec<i64> =
            context.get_furnitures(pid).await.unwrap().iter().map(|f| f.api_id).collect();
        assert_eq!(furniture, [2, 39, 73, 103, 134, 165, 500]);
        assert_eq!(basic.api_furniture, [2, 39, 73, 103, 134, 165]);

        let options = context.get_option_settings(pid).await.unwrap().unwrap();
        assert_eq!(options.api_skin_id, 311);
        let game = context.get_game_settings(pid).await.unwrap();
        assert_eq!((game.api_position_id, game.api_p_bgm_id), (1, 140));
        let oss = context.get_oss_settings(pid).await.unwrap();
        assert_eq!((oss.api_language_type, oss.api_oss_items), (1, [0, 0, 0, 0, 1, 0, 0, 0]));
    }

    #[tokio::test]
    async fn ships_and_equipment_get_local_ids_and_keep_their_owners_and_order() {
        let context = crate::TestContext::new().await;
        // Another profile's ship and equipment take the first ids, so a live id written
        // as is would land on someone else's row.
        let other = new_profile(&context, "import-other").await;
        context.add_ship(other, 951).await.unwrap();
        let pid = new_profile(&context, "import-ships").await;
        context.import_account(pid, &snapshot()).await.unwrap();

        let ships = context.get_ships(pid).await.unwrap();
        // Ascending live id: 7 (天津風改二) first, then 50 (鳳翔).
        let order: Vec<i64> = ships.iter().map(|s| s.api_ship_id).collect();
        assert_eq!(order, [951, 89]);
        assert!(ships.iter().all(|s| s.api_id != 7 && s.api_id != 50), "live ids reused");

        let items = context.get_slot_items(pid).await.unwrap();
        let mst_of =
            |id: i64| items.iter().find(|item| item.api_id == id).map(|item| item.api_slotitem_id);
        let (amatsukaze, houshou) = (&ships[0], &ships[1]);
        assert_eq!(mst_of(amatsukaze.api_slot[0]), Some(2));
        assert_eq!(mst_of(amatsukaze.api_slot_ex), Some(43));
        assert_eq!(mst_of(houshou.api_slot[0]), Some(19));
        assert!(items.iter().all(|item| ![12, 77, 300].contains(&item.api_id)), "live ids reused");

        let fighter = items.iter().find(|item| item.api_slotitem_id == 19).unwrap();
        assert_eq!(fighter.api_alv, Some(7));
        let gun = items.iter().find(|item| item.api_slotitem_id == 2).unwrap();
        assert_eq!((gun.api_level, gun.api_locked), (6, 1));

        let unset = context.get_unset_slot_items(pid).await.unwrap();
        assert!(unset.is_empty(), "every piece is carried, so none is unset: {unset:?}");

        let sash = houshou.api_sp_effect_items.as_ref().expect("the sash is imported");
        assert_eq!((sash[0].api_kind, sash[0].api_kaih, sash[0].api_souk), (2, Some(2), Some(1)));
    }

    #[tokio::test]
    async fn fleets_and_docks_point_at_the_local_ships() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "import-fleets").await;
        context.import_account(pid, &snapshot()).await.unwrap();

        let ships = context.get_ships(pid).await.unwrap();
        let (amatsukaze, houshou) = (ships[0].api_id, ships[1].api_id);

        let fleets = context.get_fleets(pid).await.unwrap();
        assert_eq!(fleets.len(), 2);
        assert_eq!((fleets[0].name.as_str(), fleets[0].ships[0]), ("第1艦隊", houshou));
        assert_eq!(fleets[1].ships[0], amatsukaze);
        let mission = fleets[1].mission.as_ref().expect("fleet 2 is on an expedition");
        assert_eq!(mission.id, 2);
        assert_eq!(mission.return_time.map(|t| t.timestamp_millis()), Some(LATER_MS));

        let ndocks = context.get_ndocks(pid).await.unwrap();
        assert_eq!(ndocks[0].status, RepairDockStatus::Busy);
        let repair = ndocks[0].context.as_ref().expect("dock 1 repairs");
        assert_eq!((repair.ship_id, repair.fuel, repair.steel), (houshou, 10, 20));
        assert_eq!(repair.complete_time.timestamp_millis(), LATER_MS);

        let kdocks = context.get_kdocks(pid).await.unwrap();
        assert_eq!(kdocks[0].status, ConstructionDockStatus::Busy);
        let build = kdocks[0].context.as_ref().expect("dock 1 builds");
        assert_eq!((build.ship_id, build.is_large), (951, true));
        assert_eq!(kdocks[1].status, ConstructionDockStatus::Locked);
    }

    #[tokio::test]
    async fn the_report_lists_a_ship_whose_live_stats_the_local_formulas_do_not_give() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "import-report").await;
        let (mut port, require_info) = fixture();
        port["api_ship"][0]["api_karyoku"] = json!([999, 999]);
        let snapshot = LiveSnapshot::from_api_data(&port, &require_info).unwrap();
        let report = context.import_account(pid, &snapshot).await.unwrap();

        assert_eq!((report.ships, report.slot_items, report.fleets), (2, 3, 2));
        let houshou = report.mismatches.iter().find(|ship| ship.live_id == 50).expect("listed");
        let firepower = houshou.fields.iter().find(|f| f.field == "api_karyoku[0]").unwrap();
        assert_eq!(firepower.live, 999);
        assert_ne!(firepower.local, 999);
        assert!(report.to_string().contains("ship 50 (mst 89)"));
    }

    #[tokio::test]
    async fn equipment_the_codex_lacks_stops_the_import_before_anything_is_written() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "import-missing").await;
        let (port, mut require_info) = fixture();
        require_info["api_slot_item"][0]["api_slotitem_id"] = json!(99_999);
        let snapshot = LiveSnapshot::from_api_data(&port, &require_info).unwrap();
        assert_eq!(missing_masters(&context.codex, &snapshot).slot_items, [99_999]);

        assert!(context.import_account(pid, &snapshot).await.is_err());
        assert!(context.get_ships(pid).await.unwrap().is_empty());
        assert!(context.get_slot_items(pid).await.unwrap().is_empty());
    }
}
