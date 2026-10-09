use axum::{Router, routing::post};

mod next;
mod projection;
mod select_eventmap_rank;
mod start;
mod start_air_base;

pub(super) fn router() -> Router {
    Router::new()
        .route("/next", post(next::handler))
        .route("/select_eventmap_rank", post(select_eventmap_rank::handler))
        .route("/start", post(start::handler))
        .route("/start_air_base", post(start_air_base::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::Pid;
    use crate::net::router::kcsapi::test_utils::{
        app_state, new_test_context, seed_single_ship_fleet,
    };
    use axum::Form;

    #[tokio::test]
    async fn start_and_next_handlers_drive_map_progression() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        let start = start::handler(
            app_state(&context.state),
            Pid(context.session.profile.id),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();
        let start_data = start.api_data.unwrap();
        assert_eq!(start_data["api_maparea_id"], 1);
        assert_eq!(start_data["api_mapinfo_no"], 1);

        let next = next::handler(
            app_state(&context.state),
            Pid(context.session.profile.id),
            Form(next::Params {
                api_recovery_type: 0,
                api_cell_id: Some(2),
            }),
        )
        .await
        .unwrap();
        let next_data = next.api_data.unwrap();
        assert_eq!(next_data["api_maparea_id"], 1);
        assert_eq!(next_data["api_mapinfo_no"], 1);
        assert_eq!(next_data["api_from_no"], start_data["api_no"]);
    }

    #[tokio::test]
    async fn start_handler_returns_exact_cells_for_world_1_1() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        let start = start::handler(
            app_state(&context.state),
            Pid(context.session.profile.id),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();
        let start_data = start.api_data.unwrap();
        let cells = start_data["api_cell_data"].as_array().unwrap();

        let api_nos = cells.iter().map(|cell| cell["api_no"].as_i64().unwrap()).collect::<Vec<_>>();
        let api_ids = cells.iter().map(|cell| cell["api_id"].as_i64().unwrap()).collect::<Vec<_>>();

        assert_eq!(api_nos, vec![0, 1, 2, 3]);
        assert_eq!(api_ids, vec![3001, 3002, 3003, 3004]);
        assert!(!cells.iter().any(|cell| cell["api_no"].as_i64() == Some(4)));
        assert!(!cells.iter().any(|cell| cell["api_id"].as_i64() == Some(1104)));
    }

    #[tokio::test]
    async fn next_handler_rejects_while_battle_result_is_pending() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        start::handler(
            app_state(&context.state),
            Pid(context.session.profile.id),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();

        context.state.sortie_battle(pid, 1).await.unwrap();

        assert!(
            next::handler(
                app_state(&context.state),
                Pid(context.session.profile.id),
                Form(next::Params {
                    api_recovery_type: 0,
                    api_cell_id: None,
                }),
            )
            .await
            .is_err()
        );
    }

    /// 6-4 open, one air corps with 一式陸攻 (radius 9) ordered to sortie, and a
    /// sortie under way.
    async fn air_corps_over_6_4(
        context: &crate::net::router::kcsapi::test_utils::TestContext,
        action: i64,
    ) -> i64 {
        use emukc_internal::gameplay::prelude::{Scenario, apply_scenario};

        let pid = context.session.profile.id;
        apply_scenario(&context.state, pid, &Scenario::air_corps_6_4()).await.unwrap();
        context.state.get_airbases(pid).await.unwrap();
        let bomber = context.state.add_slot_item(pid, 169, 0, 0).await.unwrap();
        context.state.set_airbase_plane(pid, 6, 1, 1, bomber.api_id).await.unwrap();
        context.state.set_airbase_actions(pid, 6, &[(1, action)]).await.unwrap();
        context.state.start_sortie(pid, 1, 6, 4).await.unwrap();
        pid
    }

    fn strike(first: &str, second: &str) -> Form<start_air_base::Params> {
        Form(start_air_base::Params {
            api_strike_point_1: first.to_string(),
            api_strike_point_2: second.to_string(),
            api_strike_point_3: String::new(),
        })
    }

    #[tokio::test]
    async fn an_air_corps_is_sent_against_cells_its_radius_reaches() {
        let context = new_test_context().await;
        let pid = air_corps_over_6_4(&context, 1).await;

        let before = context.state.get_materials(pid).await.unwrap();
        start_air_base::handler(app_state(&context.state), Pid(pid), strike("13,13", ""))
            .await
            .unwrap();
        assert!(
            start_air_base::handler(app_state(&context.state), Pid(pid), strike("13,13", ""))
                .await
                .is_err(),
            "an air corps is sent once a sortie"
        );

        let after = context.state.get_materials(pid).await.unwrap();
        assert_eq!(before.fuel - after.fuel, 27, "eighteen 陸攻 fly on 27 fuel");
        assert_eq!(before.ammo - after.ammo, 12, "and 12 ammunition");

        let active = context.state.sortie_store.get_active(pid).unwrap();
        assert_eq!(active.air_strikes.len(), 1);
        assert_eq!(active.air_strikes[0].base_rid, 1);
        assert_eq!(active.air_strikes[0].cells, [13, 13], "the same cell twice is two attacks");
    }

    #[tokio::test]
    async fn a_strike_the_air_corps_cannot_fly_is_refused() {
        let context = new_test_context().await;
        let pid = air_corps_over_6_4(&context, 1).await;
        let send = |first: &'static str, second: &'static str| {
            start_air_base::handler(app_state(&context.state), Pid(pid), strike(first, second))
        };

        assert!(send("1,2,3", "").await.is_err(), "a corps flies two cells at most");
        assert!(send("0", "").await.is_err(), "the start has no distance and is not offered");
        assert!(send("", "1").await.is_err(), "there is no second air corps");

        // A second corps exists now, but 6-4 lets one sortie.
        context.state.add_use_item(pid, 73, 1).await.unwrap();
        context.state.expand_airbase(pid, 6).await.unwrap();
        let fighter = context.state.add_slot_item(pid, 175, 0, 0).await.unwrap();
        context.state.set_airbase_plane(pid, 6, 2, 1, fighter.api_id).await.unwrap();
        context.state.set_airbase_actions(pid, 6, &[(2, 1)]).await.unwrap();
        assert!(send("1", "1").await.is_err(), "6-4 lets one air corps sortie");

        // 雷電 reaches 2; cell 13 is 8 away.
        context.state.set_airbase_actions(pid, 6, &[(1, 0)]).await.unwrap();
        assert!(send("", "13").await.is_err(), "out of the fighter's reach");
        assert!(send("1", "").await.is_err(), "the first corps is no longer ordered out");
        send("", "1").await.unwrap();
    }

    /// The whole round: an air corps pointed twice at the cell the fleet lands on
    /// attacks before the battle, comes home short of aircraft, and a resupply
    /// pays for exactly what it lost.
    #[tokio::test]
    async fn an_air_corps_attacks_its_cell_and_comes_home_short() {
        const SEED: u64 = 9;

        let context = new_test_context().await;
        // Seeded before the sortie starts: the enemy the fleet meets is rolled
        // on the way, and most of 6-4's first fleets cannot hit a squadron.
        emukc_internal::crypto::rng::seed(SEED);
        let pid = air_corps_over_6_4(&context, 1).await;
        let cell = context.state.sortie_store.get_active(pid).unwrap().current_cell_id;
        let target = format!("{cell},{cell}");

        start_air_base::handler(app_state(&context.state), Pid(pid), strike(&target, ""))
            .await
            .unwrap();

        let battle = context.state.sortie_battle(pid, 1).await.unwrap();
        context.state.sortie_battle_result(pid).await.unwrap();
        emukc_internal::crypto::rng::reseed_from_entropy();

        let attacks = &battle.api_air_base_attack;
        assert!(!attacks.is_empty(), "the battle opens with the air corps");
        assert_eq!(attacks[0].api_base_id, 1);
        assert_eq!(attacks[0].api_squadron_plane[0].api_mst_id, 169);
        assert_eq!(attacks[0].api_squadron_plane[0].api_count, 18);
        let json = serde_json::to_value(&battle).unwrap();
        assert!(
            json["api_air_base_attack"][0]["api_stage1"].is_object(),
            "stages sit beside the id"
        );
        assert!(json["api_air_base_attack"][0].get("remaining").is_none());

        // The rules read out of the client accept the packet as a day battle.
        let assets =
            emukc_internal::bootstrap::prelude::load_repo_battle_knowledge_assets().unwrap();
        let report = emukc_internal::bootstrap::prelude::validate_day_battle_response(
            &context.state.codex.manifest,
            &battle,
            &assets,
        )
        .unwrap();
        assert!(!report.has_errors(), "validation findings: {:?}", report.findings);

        let left = attacks.last().unwrap().remaining[0].1;
        assert!(left < 18, "this seed costs the squadron aircraft; pick another if it stops");
        let planes = squadrons_of_air_corps(&context, pid).await;
        assert_eq!(planes[0].count, left, "what the last attack left is what was written back");

        let before = context.state.get_materials(pid).await.unwrap();
        let supplied = context.state.supply_airbase(pid, 6, 1, &[1]).await.unwrap();
        assert_eq!(supplied.updated[0].count, 18);
        assert_eq!(supplied.after_fuel, before.fuel - (18 - left) * 3);
        assert_eq!(supplied.after_bauxite, before.bauxite - (18 - left) * 5);
    }

    async fn squadrons_of_air_corps(
        context: &crate::net::router::kcsapi::test_utils::TestContext,
        pid: i64,
    ) -> Vec<emukc_internal::model::profile::airbase::PlaneInfo> {
        context.state.get_airbases(pid).await.unwrap().remove(0).planes
    }
}
