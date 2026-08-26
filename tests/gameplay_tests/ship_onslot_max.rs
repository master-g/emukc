//! Tests for hangar expansion state (`onslot_plus_*` columns) and the derived
//! `api_onslot_max` output (KTD1: DB increments are the single source of truth,
//! the API field is synthesized in the gameplay read wrappers only).

#[cfg(test)]
mod tests {
    use emukc_internal::db::sea_orm::{
        ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel,
    };
    use emukc_internal::prelude::*;

    /// 赤城, api_maxeq = [18, 18, 27, 10, 0]
    const AKAGI_MST_ID: i64 = 83;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-onslot-max", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "onslot-max-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    /// Write expansion increments straight to the DB columns (the fact source).
    async fn set_plus(context: &crate::TestContext, ship_id: i64, plus: [Option<i64>; 5]) {
        use emukc_internal::db::entity::profile::ship;

        let m = ship::Entity::find_by_id(ship_id).one(context.db()).await.unwrap().unwrap();
        let mut am = m.into_active_model();
        am.onslot_plus_1 = ActiveValue::Set(plus[0]);
        am.onslot_plus_2 = ActiveValue::Set(plus[1]);
        am.onslot_plus_3 = ActiveValue::Set(plus[2]);
        am.onslot_plus_4 = ActiveValue::Set(plus[3]);
        am.onslot_plus_5 = ActiveValue::Set(plus[4]);
        am.update(context.db()).await.unwrap();
    }

    async fn plus_of(context: &crate::TestContext, ship_id: i64) -> [Option<i64>; 5] {
        use emukc_internal::db::entity::profile::ship;

        let m = ship::Entity::find_by_id(ship_id).one(context.db()).await.unwrap().unwrap();
        [m.onslot_plus_1, m.onslot_plus_2, m.onslot_plus_3, m.onslot_plus_4, m.onslot_plus_5]
    }

    #[tokio::test]
    async fn new_ship_omits_onslot_max() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();

        assert_eq!(ship.api_onslot_max, None);
        let json = serde_json::to_string(&ship).unwrap();
        assert!(!json.contains("api_onslot_max"), "field must be absent: {json}");

        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot_max, None);
        assert_eq!(plus_of(&context, ship.api_id).await, [None; 5]);
    }

    #[tokio::test]
    async fn all_null_plus_omits_field() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();

        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot_max, None);
        let json = serde_json::to_string(&found).unwrap();
        assert!(!json.contains("api_onslot_max"), "field must be absent: {json}");
    }

    #[tokio::test]
    async fn all_zero_plus_omits_field() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        set_plus(&context, ship.api_id, [Some(0); 5]).await;

        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot_max, None, "all-zero increments must omit the field");
    }

    #[tokio::test]
    async fn partial_plus_synthesizes_maxeq_plus_increment() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        // 赤城 manifest api_maxeq = [18, 18, 27, 10, 0]
        set_plus(&context, ship.api_id, [Some(2), None, Some(1), None, None]).await;

        // find_ship read wrapper
        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot_max, Some([18 + 2, 18, 27 + 1, 10, 0]));

        // get_ships read wrapper (port path)
        let ships = context.get_ships(pid).await.unwrap();
        let found = ships.iter().find(|s| s.api_id == ship.api_id).unwrap();
        assert_eq!(found.api_onslot_max, Some([20, 18, 28, 10, 0]));
    }

    #[tokio::test]
    async fn update_ship_roundtrip_preserves_plus() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        set_plus(&context, ship.api_id, [Some(3), Some(1), None, None, Some(2)]).await;

        // find -> mutate a regular field -> update, exactly like supply etc.
        let mut found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot_max, Some([21, 19, 27, 10, 2]));
        found.api_cond = 85;
        context.update_ship(&found).await.unwrap();

        // increments survive the roundtrip and the synthesized value is not
        // written back into the increment columns (no double-add)
        assert_eq!(plus_of(&context, ship.api_id).await, [Some(3), Some(1), None, None, Some(2)]);
        let again = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(again.api_onslot_max, Some([21, 19, 27, 10, 2]));
        assert_eq!(again.api_cond, 85);
    }
}
