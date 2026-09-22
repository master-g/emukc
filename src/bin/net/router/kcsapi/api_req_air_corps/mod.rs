use axum::{Router, routing::post};

mod change_deployment_base;
mod set_plane;

pub(super) fn router() -> Router {
    Router::new()
        .route("/change_deployment_base", post(change_deployment_base::handler))
        .route("/set_plane", post(set_plane::handler))
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
    /// 瑞雲, a seaplane bomber: it has a radius but no land base flies it.
    const SEAPLANE_BOMBER: i64 = 26;
    /// 二式大艇, a flying boat — one aircraft, not eighteen.
    const FLYING_BOAT: i64 = 138;

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
        assert!(
            data.get("api_after_bauxite").is_none(),
            "assignment is free until the resupply cost is settled"
        );

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

        // Reading the airbases settles the relocation and empties the slot.
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
            let updated = resp.api_data.unwrap()["api_plane_info"].as_array().unwrap().len();
            let expected = if squadron_id == 1 {
                1
            } else {
                2
            };
            assert_eq!(updated, expected, "a move reports the slot it left as well");
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

        let seaplane = context.state.add_slot_item(pid, SEAPLANE_BOMBER, 0, 0).await.unwrap();
        let err = set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 1,
                api_item_id: seaplane.api_id,
            }),
        )
        .await;
        assert!(err.is_err(), "a seaplane bomber has a radius but no land base flies it");

        // A flying boat may fly, but only one of it.
        let boat = context.state.add_slot_item(pid, FLYING_BOAT, 0, 0).await.unwrap();
        set_plane::handler(
            app_state(&context.state),
            Pid(pid),
            Form(set_plane::Params {
                api_area_id: 6,
                api_base_id: 1,
                api_squadron_id: 1,
                api_item_id: boat.api_id,
            }),
        )
        .await
        .unwrap();
        let planes = squadrons(&context.state, pid, 1).await;
        assert_eq!(planes[0].api_count, Some(1), "a flying boat squadron is a single aircraft");
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
}
