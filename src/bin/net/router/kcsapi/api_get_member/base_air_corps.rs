use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let airbases = state.get_airbases(pid).await?;
    let resp: Vec<KcApiAirBase> = airbases.into_iter().map(std::convert::Into::into).collect();

    Ok(KcApiResponse::success(&resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};

    /// A fresh profile has cleared nothing, so no area entitles it to an air
    /// corps yet.
    #[tokio::test]
    async fn fresh_profile_has_no_air_corps() {
        let context = new_test_context().await;
        let resp =
            handler(app_state(&context.state), Pid(context.session.profile.id)).await.unwrap();
        let data = resp.api_data.unwrap();

        assert!(data.as_array().unwrap().is_empty());
    }

    /// An air corps reports every slot it has, occupied or not, and an empty
    /// slot carries neither a count nor a condition — `docs/apilist.txt` marks
    /// `api_count`, `api_max_count` and `api_cond` as 未配属なら存在しない.
    #[tokio::test]
    async fn an_empty_air_corps_reports_four_bare_slots() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        context.state.unlock_airbase(pid, 6, 1).await.unwrap();

        let resp = handler(app_state(&context.state), Pid(pid)).await.unwrap();
        let data = resp.api_data.unwrap();
        let bases = data.as_array().unwrap();

        assert_eq!(bases.len(), 1);
        let base = &bases[0];
        assert_eq!(base["api_area_id"], 6);
        assert_eq!(base["api_rid"], 1);
        assert_eq!(base["api_action_kind"], 0, "a new air corps stands by");
        assert_eq!(base["api_distance"]["api_base"], 0, "an empty air corps reaches nowhere");

        let planes = base["api_plane_info"].as_array().unwrap();
        assert_eq!(planes.len(), 4, "the client draws a fixed four rows");
        for (index, plane) in planes.iter().enumerate() {
            let squadron_id = i64::try_from(index).unwrap() + 1;
            assert_eq!(plane["api_squadron_id"], squadron_id, "squadron ids are 1-based");
            assert_eq!(plane["api_state"], 0);
            assert_eq!(plane["api_slotid"], 0);
            assert!(plane.get("api_count").is_none(), "an empty slot has no count");
            assert!(plane.get("api_max_count").is_none());
            assert!(plane.get("api_cond").is_none());
        }
    }
}
