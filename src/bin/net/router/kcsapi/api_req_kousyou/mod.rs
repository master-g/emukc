use axum::{Router, routing::post};

mod createitem;
mod createship;
mod createship_speedchange;
mod destroyitem2;
mod destroyship;
mod getship;
mod open_new_dock;
mod preset_dev_items_delete;
mod preset_dev_items_expand;
mod preset_dev_items_register;
mod preset_dev_items_update_name;
mod remodel_slot;
mod remodel_slotlist;
mod remodel_slotlist_detail;

pub(super) fn router() -> Router {
    Router::new()
        .route("/createitem", post(createitem::handler))
        .route("/createship", post(createship::handler))
        .route("/createship_speedchange", post(createship_speedchange::handler))
        .route("/destroyitem2", post(destroyitem2::handler))
        .route("/destroyship", post(destroyship::handler))
        .route("/getship", post(getship::handler))
        .route("/open_new_dock", post(open_new_dock::handler))
        .route("/preset_dev_items_register", post(preset_dev_items_register::handler))
        .route("/preset_dev_items_delete", post(preset_dev_items_delete::handler))
        .route("/preset_dev_items_update_name", post(preset_dev_items_update_name::handler))
        .route("/preset_dev_items_expand", post(preset_dev_items_expand::handler))
        .route("/remodel_slotlist", post(remodel_slotlist::handler))
        .route("/remodel_slotlist_detail", post(remodel_slotlist_detail::handler))
        .route("/remodel_slot", post(remodel_slot::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::Pid;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};
    use crate::state::State;
    use axum::Form;
    use emukc_internal::prelude::*;

    /// 明石, whose presence as flagship opens the arsenal.
    const AKASHI: i64 = 182;
    /// 睦月, a secretary the 12cm単装砲 recipes accept on every weekday.
    const MUTSUKI: i64 = 1;
    /// 12cm単装砲 — improvable, and its own recipe eats a spare copy.
    const GUN_12CM: i64 = 1;

    async fn seed_arsenal(state: &std::sync::Arc<State>, pid: i64, second_mst_id: i64) {
        let akashi = state.add_ship(pid, AKASHI).await.unwrap();
        let second = state.add_ship(pid, second_mst_id).await.unwrap();
        state
            .update_fleet_ships(pid, 1, &[akashi.api_id, second.api_id, -1, -1, -1, -1])
            .await
            .unwrap();
        state
            .add_material(
                pid,
                &[
                    (MaterialCategory::Fuel, 10_000),
                    (MaterialCategory::Ammo, 10_000),
                    (MaterialCategory::Steel, 10_000),
                    (MaterialCategory::Bauxite, 10_000),
                    (MaterialCategory::DevMat, 100),
                    (MaterialCategory::Screw, 100),
                ],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn arsenal_is_closed_without_akashi_as_flagship() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        // 睦月 as flagship instead of 明石.
        seed_arsenal(&context.state, pid, MUTSUKI).await;
        let mutsuki = context.state.add_ship(pid, MUTSUKI).await.unwrap();
        context
            .state
            .update_fleet_ships(pid, 1, &[mutsuki.api_id, -1, -1, -1, -1, -1])
            .await
            .unwrap();

        let err = remodel_slotlist::handler(app_state(&context.state), Pid(pid)).await;
        assert!(err.is_err(), "the arsenal must reject a non-明石 flagship");
    }

    #[tokio::test]
    async fn slotlist_offers_recipes_the_second_ship_unlocks() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_arsenal(&context.state, pid, MUTSUKI).await;

        let resp = remodel_slotlist::handler(app_state(&context.state), Pid(pid)).await.unwrap();
        let rows = resp.api_data.unwrap();
        let rows = rows.as_array().unwrap();

        assert!(!rows.is_empty(), "睦月 unlocks at least one recipe every day");
        assert!(
            rows.iter().any(|row| row["api_slot_id"] == GUN_12CM),
            "12cm単装砲 must be improvable with 睦月 as second ship"
        );
        // Every row carries the full cost shape the client reads.
        for row in rows {
            for key in ["api_id", "api_req_fuel", "api_req_buildkit", "api_req_remodelkit"] {
                assert!(row.get(key).is_some(), "row is missing {key}");
            }
            assert_eq!(row["api_sp_type"], 0);
        }
    }

    /// A guaranteed attempt must succeed, add exactly one star, and eat the
    /// spare copy the recipe demands.
    #[tokio::test]
    async fn certain_improvement_adds_a_star_and_eats_the_required_copy() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_arsenal(&context.state, pid, MUTSUKI).await;

        let target = context.state.add_slot_item(pid, GUN_12CM, 0, 0).await.unwrap();
        let spare = context.state.add_slot_item(pid, GUN_12CM, 0, 0).await.unwrap();

        let list = remodel_slotlist::handler(app_state(&context.state), Pid(pid)).await.unwrap();
        let rows = list.api_data.unwrap();
        let recipe_id = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["api_slot_id"] == GUN_12CM)
            .expect("12cm単装砲 recipe")["api_id"]
            .as_i64()
            .unwrap();

        let detail = remodel_slotlist_detail::handler(
            app_state(&context.state),
            Pid(pid),
            Form(remodel_slotlist_detail::Params {
                api_id: recipe_id,
                api_slot_id: target.api_id,
            }),
        )
        .await
        .unwrap();
        let detail = detail.api_data.unwrap();
        assert_eq!(detail["api_req_slot_id"], GUN_12CM, "the recipe eats its own kind");
        assert_eq!(detail["api_req_slot_num"], 1);
        assert!(
            detail["api_certain_remodelkit"].as_i64().unwrap()
                >= detail["api_req_remodelkit"].as_i64().unwrap(),
            "guaranteeing an attempt is never cheaper"
        );

        let resp = remodel_slot::handler(
            app_state(&context.state),
            Pid(pid),
            Form(remodel_slot::Params {
                api_id: recipe_id,
                api_slot_id: target.api_id,
                api_certain_flag: true,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();

        assert_eq!(data["api_remodel_flag"], 1, "a guaranteed attempt cannot fail");
        assert_eq!(data["api_after_slot"]["api_level"], 1, "★0 becomes ★1");
        assert_eq!(data["api_remodel_id"][0], GUN_12CM);
        assert_eq!(data["api_remodel_id"][1], GUN_12CM, "a plain improvement keeps the type");
        assert_eq!(
            data["api_use_slot_id"].as_array().unwrap(),
            &vec![serde_json::json!(spare.api_id)],
            "the spare copy is the one consumed, not the target"
        );

        // The target survived with one more star; the spare is gone.
        let improved = context.state.find_slot_item(target.api_id).await.unwrap();
        assert_eq!(improved.api_level, 1);
        assert!(
            context.state.find_slot_item(spare.api_id).await.is_err(),
            "the consumed copy must be deleted"
        );
    }

    /// At ★10 a variant recipe swaps the equipment for a different one. The
    /// old instance is gone and the client is handed a new instance id.
    #[tokio::test]
    async fn upgrade_at_max_stars_swaps_the_equipment_for_its_variant() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_arsenal(&context.state, pid, MUTSUKI).await;

        // 12cm単装砲 at ★10 upgrades into 12cm単装高角砲(後期型) (293), eating a
        // 12.7cm連装砲 (28) rather than another copy of itself.
        let target = context.state.add_slot_item(pid, GUN_12CM, 10, 0).await.unwrap();
        let fodder = context.state.add_slot_item(pid, 28, 0, 0).await.unwrap();

        let list = remodel_slotlist::handler(app_state(&context.state), Pid(pid)).await.unwrap();
        let rows = list.api_data.unwrap();
        let recipe_id = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["api_slot_id"] == GUN_12CM)
            .expect("12cm単装砲 recipe")["api_id"]
            .as_i64()
            .unwrap();

        let detail = remodel_slotlist_detail::handler(
            app_state(&context.state),
            Pid(pid),
            Form(remodel_slotlist_detail::Params {
                api_id: recipe_id,
                api_slot_id: target.api_id,
            }),
        )
        .await
        .unwrap();
        let detail = detail.api_data.unwrap();
        assert_eq!(detail["api_change_flag"], 1, "★10 on a variant recipe is an upgrade");
        assert_eq!(detail["api_req_slot_id"], 28, "the upgrade eats a different equipment");

        let resp = remodel_slot::handler(
            app_state(&context.state),
            Pid(pid),
            Form(remodel_slot::Params {
                api_id: recipe_id,
                api_slot_id: target.api_id,
                api_certain_flag: true,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();

        assert_eq!(data["api_remodel_flag"], 1);
        assert_eq!(data["api_remodel_id"][0], GUN_12CM);
        assert_eq!(data["api_remodel_id"][1], 293, "the equipment becomes its variant");
        assert_eq!(data["api_after_slot"]["api_slotitem_id"], 293);
        assert_eq!(data["api_after_slot"]["api_level"], 0, "the variant starts at ★0");
        assert_eq!(
            data["api_use_slot_id"].as_array().unwrap(),
            &vec![serde_json::json!(fodder.api_id)]
        );

        assert!(
            context.state.find_slot_item(target.api_id).await.is_err(),
            "the pre-upgrade instance is replaced, not kept"
        );
    }

    /// Improving an equipment someone is wearing is rejected — the client is
    /// expected to unequip first.
    #[tokio::test]
    async fn equipped_items_cannot_be_improved() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_arsenal(&context.state, pid, MUTSUKI).await;

        let target = context.state.add_slot_item(pid, GUN_12CM, 0, 0).await.unwrap();
        context.state.add_slot_item(pid, GUN_12CM, 0, 0).await.unwrap();
        let akashi = context.state.get_fleet_ships(pid, 1).await.unwrap()[0].id;
        context.state.update_slot_item(target.api_id, None, None, Some(akashi)).await.unwrap();

        let list = remodel_slotlist::handler(app_state(&context.state), Pid(pid)).await.unwrap();
        let rows = list.api_data.unwrap();
        let recipe_id = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["api_slot_id"] == GUN_12CM)
            .expect("12cm単装砲 recipe")["api_id"]
            .as_i64()
            .unwrap();

        let err = remodel_slot::handler(
            app_state(&context.state),
            Pid(pid),
            Form(remodel_slot::Params {
                api_id: recipe_id,
                api_slot_id: target.api_id,
                api_certain_flag: true,
            }),
        )
        .await;

        assert!(err.is_err(), "an equipped item must not be improvable");
    }
}
