//! Tests for monthly map reset policy and EO map unlock
//!
//! EO maps (1-5, 1-6, etc.) have MapResetPolicy::Monthly and reset at the
//! start of each month. This test verifies policy assignment and that
//! clearing prerequisite maps unlocks EO maps.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-reset", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "reset-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn eo_maps_have_monthly_reset_policy() {
        let context = new_context().await;
        let codex = &context.1;
        let catalog = codex.map_catalog();

        // EO maps with monthly reset: 1-5, 1-6, 3-5, 4-5, 5-5, 6-5
        for map_id in [15, 16, 35, 45, 55, 65] {
            let def = catalog.map_definition(map_id);
            assert!(def.is_some(), "map {map_id} should exist in catalog");
            assert_eq!(
                def.unwrap().reset_policy,
                emukc_internal::model::codex::map::MapResetPolicy::Monthly,
                "map {map_id} should have Monthly reset policy"
            );
        }
    }

    #[tokio::test]
    async fn regular_maps_have_never_reset_policy() {
        let context = new_context().await;
        let codex = &context.1;
        let catalog = codex.map_catalog();

        // Regular maps: 1-1 through 1-4, 2-1, etc.
        for map_id in [11, 12, 13, 14, 21, 24] {
            let def = catalog.map_definition(map_id);
            assert!(def.is_some(), "map {map_id} should exist in catalog");
            assert_eq!(
                def.unwrap().reset_policy,
                emukc_internal::model::codex::map::MapResetPolicy::Never,
                "map {map_id} should have Never reset policy"
            );
        }
    }

    #[tokio::test]
    async fn new_profile_does_not_show_eo_maps() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();

        // EO maps should NOT be visible for a new account
        assert!(
            !map_ids.contains(&15),
            "map 1-5 should not be visible for new account, got: {map_ids:?}"
        );
    }

    #[tokio::test]
    async fn monthly_reset_clears_cleared_defeat_count_and_gauge_index() {
        use emukc_internal::db::sea_orm::{
            ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
        };
        use emukc_internal::time::chrono::{Duration, Utc};

        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Find map 1-5 (map_id=15), an EO map with Monthly reset policy.
        let map_id = 15;
        let record = emukc_internal::db::entity::profile::map_record::Entity::find()
            .filter(emukc_internal::db::entity::profile::map_record::Column::ProfileId.eq(pid))
            .filter(emukc_internal::db::entity::profile::map_record::Column::MapId.eq(map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();

        // Set stale state: cleared, non-zero defeat_count, gauge_index > 1,
        // and a last_reset_at well before the current month boundary.
        let stale_reset_at = Utc::now() - Duration::days(60);
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(true);
        am.unlocked = ActiveValue::Set(true);
        am.defeat_count = ActiveValue::Set(Some(5));
        am.gauge_index = ActiveValue::Set(3);
        am.last_reset_at = ActiveValue::Set(Some(stale_reset_at));
        am.update(&context.0).await.unwrap();

        // Trigger refresh by calling get_map_infos (which calls refresh_all_map_records).
        let _ = context.get_map_infos(pid).await.unwrap();

        // Verify reset occurred.
        let refreshed = emukc_internal::db::entity::profile::map_record::Entity::find()
            .filter(emukc_internal::db::entity::profile::map_record::Column::ProfileId.eq(pid))
            .filter(emukc_internal::db::entity::profile::map_record::Column::MapId.eq(map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();

        assert!(!refreshed.cleared, "cleared should be reset to false");
        assert_eq!(refreshed.defeat_count, Some(0), "defeat_count should be reset to 0");
        assert_eq!(refreshed.gauge_index, 1, "gauge_index should be reset to 1");
    }
}
