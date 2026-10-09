use axum::{Router, routing::post};

mod change_deployment_base;
mod change_name;
mod cond_recovery;
mod expand_base;
mod expand_maintenance_level;
mod set_action;
mod set_plane;
mod supply;

pub(super) fn router() -> Router {
    Router::new()
        .route("/change_deployment_base", post(change_deployment_base::handler))
        .route("/change_name", post(change_name::handler))
        .route("/cond_recovery", post(cond_recovery::handler))
        .route("/expand_base", post(expand_base::handler))
        .route("/expand_maintenance_level", post(expand_maintenance_level::handler))
        .route("/set_action", post(set_action::handler))
        .route("/set_plane", post(set_plane::handler))
        .route("/supply", post(supply::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::Pid;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};
    use crate::state::State;
    use axum::Form;
    use emukc_internal::prelude::*;

    /// 一式陸攻, a land-based bomber: radius 9, a full 18-plane squadron.
    const BOMBER: i64 = 169;
    /// 雷電, a local fighter with the shortest radius of the pair.
    const FIGHTER: i64 = 175;
    /// 瑞雲, a seaplane bomber; the client's list offers it like any bomber.
    const SEAPLANE_BOMBER: i64 = 26;
    /// 二式大艇, a flying boat — four aircraft, not eighteen.
    const FLYING_BOAT: i64 = 138;
    /// 12cm単装砲.
    const MAIN_GUN: i64 = 1;

    async fn seed_airbase(state: &std::sync::Arc<State>, pid: i64, rid: i64) {
        state.unlock_airbase(pid, 6, rid).await.unwrap();
    }

    async fn squadrons(state: &std::sync::Arc<State>, pid: i64, rid: i64) -> Vec<KcApiPlaneInfo> {
        let bases = state.get_airbases(pid).await.unwrap();
        let base: KcApiAirBase = bases.into_iter().find(|b| b.rid == rid).expect("airbase").into();
        base.api_plane_info
    }

    #[tokio::test]
    async fn assigning_a_squadron_fills_the_slot_and_sets_the_radius() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        let before = context.state.get_materials(pid).await.unwrap().bauxite;

        let resp = set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 1,
                api_item_id: bomber.api_id,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();

        assert_eq!(data["api_distance"]["api_base"], 9, "the base reaches as far as its bomber");
        let updated = data["api_plane_info"].as_array().unwrap();
        assert_eq!(updated.len(), 1, "a plain assignment reports one slot");
        assert_eq!(updated[0]["api_squadron_id"], 1);
        assert_eq!(updated[0]["api_state"], 1);
        assert_eq!(updated[0]["api_slotid"], bomber.api_id);
        assert_eq!(updated[0]["api_count"], 18, "a bomber squadron flies eighteen");
        assert_eq!(updated[0]["api_max_count"], 18);
        assert_eq!(data["api_after_bauxite"], before - 216, "eighteen bombers at 12 bauxite each");

        // A second, shorter-legged squadron drags the radius down.
        let fighter = context.state.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap();
        let resp = set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 2,
                api_item_id: fighter.api_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            resp.api_data.unwrap()["api_distance"]["api_base"],
            2,
            "the radius is the shortest squadron's, not the longest"
        );
    }

    /// Removal is two-phase, the way the live server answers it: the slot keeps
    /// its `api_slotid` under `api_state: 2` until the relocation settles.
    #[tokio::test]
    async fn clearing_a_slot_relocates_before_it_empties() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        let mut last = None;
        for item_id in [bomber.api_id, -1] {
            last = set_plane::handler(
                app_state(&context.state),
                Pid(pid),
                Form(set_plane::Params {
                    api_area_id: 6,
                    api_base_id: 1,
                    api_squadron_id: 1,
                    api_item_id: item_id,
                }),
            )
            .await
            .unwrap()
            .api_data;
        }

        let removal = last.unwrap();
        let slot = &removal["api_plane_info"].as_array().unwrap()[0];
        assert_eq!(slot["api_state"], 2, "removal reports the slot as relocating");
        assert_eq!(slot["api_slotid"], bomber.api_id, "and keeps the equipment on it");
        assert!(slot.get("api_count").is_none(), "a relocating slot carries no count");
        assert!(slot.get("api_cond").is_none());
        assert_eq!(
            removal["api_distance"]["api_base"], 0,
            "the airbase reaches nowhere once its only squadron left"
        );

        // The slot stays that way through reads of the airbases; once twelve
        // minutes are up, the port lets the squadron go and the slot empties.
        let planes = squadrons(&context.state, pid, 1).await;
        assert_eq!(planes[0].api_state, 2);
        {
            use emukc_internal::db::{
                entity::profile::airbase::plane,
                sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel},
            };
            use emukc_internal::time::chrono::{Duration, Utc};

            let db = &*context.state.db;
            let row = plane::Entity::find_by_id(bomber.api_id).one(db).await.unwrap().unwrap();
            let mut am = row.into_active_model();
            am.since = ActiveValue::Set(Some(Utc::now() - Duration::minutes(12)));
            am.update(db).await.unwrap();
        }
        let port = context.state.port_view(pid).await.unwrap();
        assert!(port.relocating_slots.is_empty());
        let planes = squadrons(&context.state, pid, 1).await;
        assert_eq!(planes[0].api_state, 0);
        assert_eq!(planes[0].api_slotid, 0);
        assert!(planes[0].api_count.is_none(), "an empty slot carries no count");
        assert!(planes[0].api_cond.is_none());

        // The equipment went back to the inventory rather than being eaten.
        assert!(context.state.find_slot_item(bomber.api_id).await.is_ok());
    }

    /// A squadron still in relocation may be assigned again, including to
    /// another airbase — it flies for nobody until it settles.
    #[tokio::test]
    async fn a_relocating_squadron_can_be_reassigned() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;
        seed_airbase(&context.state, pid, 2).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        let assign = |base_id, item_id| {
            set_plane::handler(
                app_state(&context.state),
                Pid(pid),
                Form(set_plane::Params {
                    api_area_id: 6,
                    api_base_id: base_id,
                    api_squadron_id: 1,
                    api_item_id: item_id,
                }),
            )
        };

        assign(1, bomber.api_id).await.unwrap();
        assign(1, -1).await.unwrap();
        let resp = assign(2, bomber.api_id).await.unwrap();

        let data = resp.api_data.unwrap();
        let slot = &data["api_plane_info"].as_array().unwrap()[0];
        assert_eq!(slot["api_state"], 1, "the second airbase took it");
        assert!(
            data.get("api_after_bauxite").is_some(),
            "landing from relocation is a deployment and is paid for"
        );
        assert_eq!(slot["api_slotid"], bomber.api_id);
    }

    #[tokio::test]
    async fn moving_within_one_airbase_reports_both_slots() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        for squadron_id in [1, 3] {
            let resp = set_plane::handler(
                app_state(&context.state),
                Pid(pid),
                Form(set_plane::Params {
                    api_area_id: 6,
                    api_base_id: 1,
                    api_squadron_id: squadron_id,
                    api_item_id: bomber.api_id,
                }),
            )
            .await
            .unwrap();
            let data = resp.api_data.unwrap();
            let updated = data["api_plane_info"].as_array().unwrap().len();
            let expected = if squadron_id == 1 {
                1
            } else {
                2
            };
            assert_eq!(updated, expected, "a move reports the slot it left as well");
            assert_eq!(
                data.get("api_after_bauxite").is_some(),
                squadron_id == 1,
                "only the deployment spends bauxite, not the move"
            );
        }

        let planes = squadrons(&context.state, pid, 1).await;
        assert_eq!(planes[0].api_state, 0, "slot 1 was vacated");
        assert_eq!(planes[2].api_state, 1, "slot 3 took the squadron");
        assert_eq!(planes[2].api_slotid, bomber.api_id);
    }

    #[tokio::test]
    async fn only_land_capable_equipment_may_be_assigned() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let gun = context.state.add_slot_item(pid, MAIN_GUN, 0, 0).await.unwrap();
        let seaplane = context.state.add_slot_item(pid, SEAPLANE_BOMBER, 0, 0).await.unwrap();
        let boat = context.state.add_slot_item(pid, FLYING_BOAT, 0, 0).await.unwrap();
        let assign = |squadron_id, item_id| {
            set_plane::handler(
                app_state(&context.state),
                Pid(pid),
                Form(set_plane::Params {
                    api_area_id: 6,
                    api_base_id: 1,
                    api_squadron_id: squadron_id,
                    api_item_id: item_id,
                }),
            )
        };

        assert!(assign(1, gun.api_id).await.is_err(), "a gun is not on the client's list");
        assign(1, seaplane.api_id).await.unwrap();
        assign(2, boat.api_id).await.unwrap();

        let planes = squadrons(&context.state, pid, 1).await;
        assert_eq!(planes[0].api_count, Some(18), "a seaplane bomber flies a full squadron");
        assert_eq!(planes[1].api_count, Some(4), "a flying boat squadron is four aircraft");
    }

    #[tokio::test]
    async fn a_deployment_change_swaps_two_airbases_squadrons() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;
        seed_airbase(&context.state, pid, 2).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        let fighter = context.state.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap();
        for (rid, item_id) in [(1, bomber.api_id), (2, fighter.api_id)] {
            set_plane::handler(
                app_state(&context.state),
                Pid(pid),
                Form(set_plane::Params {
                    api_area_id: 6,
                    api_base_id: rid,
                    api_squadron_id: 1,
                    api_item_id: item_id,
                }),
            )
            .await
            .unwrap();
        }

        // Move the bomber onto base 2's occupied slot: the two trade places.
        let resp = change_deployment_base::handler(
            app_state(&context.state),
            Pid(pid),
            Form(change_deployment_base::Params {
                api_area_id: 6,
                api_base_id: 2,
                api_base_id_src: 1,
                api_squadron_id: 1,
                api_item_id: bomber.api_id,
            }),
        )
        .await
        .unwrap();
        let items = resp.api_data.unwrap();
        let items = items["api_base_items"].as_array().unwrap();
        assert_eq!(items.len(), 2, "both airbases come back whole");

        assert_eq!(squadrons(&context.state, pid, 1).await[0].api_slotid, fighter.api_id);
        assert_eq!(squadrons(&context.state, pid, 2).await[0].api_slotid, bomber.api_id);
    }

    #[tokio::test]
    async fn orders_and_a_name_reach_the_airbases() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;
        seed_airbase(&context.state, pid, 2).await;

        set_action::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_action::Params {
                api_area_id: 6,
                api_base_id: vec![1, 2],
                api_action_kind: vec![2, 1],
            }),
        )
        .await
        .unwrap();
        change_name::handler(
            app_state(&context.state),
            Pid(pid),
            Form(change_name::Params {
                api_area_id: 6,
                api_base_id: 2,
                api_name: "\u{9678}\u{653B}\u{968A}".to_string(),
            }),
        )
        .await
        .unwrap();

        let bases: Vec<KcApiAirBase> = context
            .state
            .get_airbases(pid)
            .await
            .unwrap()
            .into_iter()
            .map(std::convert::Into::into)
            .collect();
        assert_eq!(bases[0].api_action_kind, 2, "the first airbase defends");
        assert_eq!(bases[1].api_action_kind, 1, "the second sorties");
        assert_eq!(bases[1].api_name, "\u{9678}\u{653B}\u{968A}");

        let err = set_action::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_action::Params {
                api_area_id: 6,
                api_base_id: vec![1],
                api_action_kind: vec![5],
            }),
        )
        .await;
        assert!(err.is_err(), "there is no fifth order");
    }

    /// 15 of 18 left: three aircraft at 3 fuel and 5 bauxite each.
    #[tokio::test]
    async fn a_resupply_fills_the_squadron_and_charges_per_aircraft() {
        use emukc_internal::db::entity::profile::airbase::plane;
        use emukc_internal::db::sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 1,
                api_item_id: bomber.api_id,
            }),
        )
        .await
        .unwrap();

        // Nothing loses aircraft yet, so take three away by hand.
        let db = context.state.db.as_ref();
        let mut am: plane::ActiveModel =
            plane::Entity::find_by_id(bomber.api_id).one(db).await.unwrap().unwrap().into();
        am.count = ActiveValue::Set(15);
        am.update(db).await.unwrap();

        let before = context.state.get_materials(pid).await.unwrap();
        let resp = supply::handler(
            app_state(&context.state),
            Pid(pid),
            Form(supply::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: vec![1, 2],
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();

        assert_eq!(data["api_after_fuel"], before.fuel - 9);
        assert_eq!(data["api_after_bauxite"], before.bauxite - 15);
        assert_eq!(data["api_distance"]["api_base"], 9, "the client reads the radius here too");
        let slots = data["api_plane_info"].as_array().unwrap();
        assert_eq!(slots.len(), 2, "both named slots come back");
        assert_eq!(slots[0]["api_count"], 18);
        assert_eq!(slots[1]["api_state"], 0, "the empty one untouched");
    }

    /// 設営隊 buys the second and third air corps, and no fourth.
    #[tokio::test]
    async fn construction_corps_add_air_corps_up_to_three() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;
        context.state.add_use_item(pid, 73, 2).await.unwrap();

        for rid in [2, 3] {
            let resp = expand_base::handler(
                app_state(&context.state),
                Pid(pid),
                Form(expand_base::Params {
                    api_area_id: 6,
                }),
            )
            .await
            .unwrap();
            let data = resp.api_data.unwrap();
            let added = data.as_array().expect("the client reads element 0");
            assert_eq!(added.len(), 1);
            assert_eq!(added[0]["api_rid"], rid);
            assert_eq!(added[0]["api_plane_info"].as_array().unwrap().len(), 4);
        }

        let bases = context.state.get_airbases(pid).await.unwrap();
        assert_eq!(
            bases.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
            [
                "\u{7B2C}\u{4E00}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}",
                "\u{7B2C}\u{4E8C}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}",
                "\u{7B2C}\u{4E09}\u{57FA}\u{5730}\u{822A}\u{7A7A}\u{968A}",
            ],
            "named the way a live account's are"
        );

        context.state.add_use_item(pid, 73, 1).await.unwrap();
        let err = expand_base::handler(
            app_state(&context.state),
            Pid(pid),
            Form(expand_base::Params {
                api_area_id: 6,
            }),
        )
        .await;
        assert!(err.is_err(), "an area holds three air corps");
        assert_eq!(
            context.state.find_use_item(pid, 73).await.unwrap().api_count,
            1,
            "and the refused one cost nothing"
        );
    }

    /// An area nobody expanded reports no 整備Lv at all; 設営隊 raise it to 3.
    #[tokio::test]
    async fn maintenance_level_is_per_area_and_stops_at_three() {
        use emukc_internal::model::profile::airbase::expanded_info;

        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;
        seed_airbase(&context.state, pid, 2).await;
        context.state.add_use_item(pid, 73, 4).await.unwrap();

        let bases = context.state.get_airbases(pid).await.unwrap();
        assert!(expanded_info(&bases).is_empty(), "no entry before the first expansion");

        let raise = || {
            expand_maintenance_level::handler(
                app_state(&context.state),
                Pid(pid),
                Form(expand_maintenance_level::Params {
                    api_area_id: 6,
                }),
            )
        };
        for _ in 0..3 {
            raise().await.unwrap();
        }
        assert!(raise().await.is_err(), "level 3 is the last");

        let bases = context.state.get_airbases(pid).await.unwrap();
        let info = expanded_info(&bases);
        assert_eq!(info.len(), 1, "one entry for the area, not one per air corps");
        assert_eq!(info[0].api_area_id, 6);
        assert_eq!(info[0].api_maintenance_level, 3);
        assert_eq!(context.state.find_use_item(pid, 73).await.unwrap().api_count, 1);
    }

    #[tokio::test]
    async fn a_ration_rests_a_tired_squadron() {
        use emukc_internal::db::entity::profile::airbase::plane;
        use emukc_internal::db::sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_airbase(&context.state, pid, 1).await;

        let bomber = context.state.add_slot_item(pid, BOMBER, 0, 0).await.unwrap();
        set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 1,
                api_item_id: bomber.api_id,
            }),
        )
        .await
        .unwrap();

        // Nothing tires a squadron yet, so do it by hand.
        let db = context.state.db.as_ref();
        let mut am: plane::ActiveModel =
            plane::Entity::find_by_id(bomber.api_id).one(db).await.unwrap().unwrap().into();
        am.condition = ActiveValue::Set(3);
        am.update(db).await.unwrap();

        let rest = || {
            cond_recovery::handler(
                app_state(&context.state),
                Pid(pid),
                Form(cond_recovery::Params {
                    api_area_id: 6,
                    api_base_id: 1,
                }),
            )
        };
        assert!(rest().await.is_err(), "resting takes a ration");

        context.state.add_use_item(pid, 102, 1).await.unwrap();
        let data = rest().await.unwrap().api_data.unwrap();
        let slots = data["api_plane_info"].as_array().unwrap();
        assert_eq!(slots.len(), 1, "only the squadron that flies");
        assert_eq!(slots[0]["api_cond"], 1);
        assert_eq!(data["api_distance"]["api_base"], 9);
        assert_eq!(context.state.find_use_item(pid, 102).await.unwrap().api_count, 0);
    }
}
