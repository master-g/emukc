//! Tests for map unlock progression system

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-unlock", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "unlock-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn find_record(
        context: &(emukc_internal::db::sea_orm::DbConn, Codex),
        pid: i64,
        map_id: i64,
    ) -> emukc_internal::db::entity::profile::map_record::Model {
        use emukc_internal::db::entity::profile::map_record;
        use emukc_internal::db::sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        map_record::Entity::find()
            .filter(map_record::Column::ProfileId.eq(pid))
            .filter(map_record::Column::MapId.eq(map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn new_profile_mapinfo_only_shows_map_1_1() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();

        assert_eq!(map_ids, vec![11], "expected only map 1-1 for new account");
    }

    #[tokio::test]
    async fn clearing_1_1_unlocks_1_2() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let catalog = context.1.map_catalog();
        let deps = catalog.dependents_of(11);
        assert!(!deps.is_empty(), "1-1 should have dependents");

        // Mark map 1-1 as cleared
        use emukc_internal::db::sea_orm::{ActiveModelTrait, ActiveValue, IntoActiveModel};
        let record = find_record(&context, pid, 11).await;
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(true);
        am.update(&context.0).await.unwrap();

        // Manually unlock dependents
        for dep_id in &deps {
            let dep_record = find_record(&context, pid, *dep_id).await;
            let mut dep_am = dep_record.into_active_model();
            dep_am.unlocked = ActiveValue::Set(true);
            dep_am.update(&context.0).await.unwrap();
        }

        // Now mapinfo should include 1-1 and 1-2
        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();
        assert!(map_ids.contains(&11), "1-1 should be visible");
        assert!(map_ids.contains(&12), "1-2 should be unlocked after clearing 1-1");
    }

    #[tokio::test]
    async fn start_sortie_to_locked_map_fails() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add ships to fleet 1
        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();

        // Attempt to sortie to 2-1 (locked for new account)
        let result = context.start_sortie(pid, 1, 2, 1, 1).await;

        assert!(result.is_err(), "sortie to locked map 2-1 should fail");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("locked"), "error should mention locked, got: {msg}",);
    }
}
