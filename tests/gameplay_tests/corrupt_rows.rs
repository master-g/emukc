//! A row that no longer matches the codex or its own enum makes the request
//! fail with an error instead of panicking the server.

#[cfg(test)]
mod tests {
    use emukc_internal::db::sea_orm::{
        ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel,
    };
    use emukc_internal::prelude::*;

    const MISSING_MST_ID: i64 = 999_999;

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, username).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn powerup_with_an_unknown_equipment_returns_an_error() {
        use emukc_internal::db::entity::profile::item::slot_item;

        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "powerup").await;

        let target = context.add_ship(pid, 1).await.unwrap();
        let material = context.add_ship(pid, 1).await.unwrap();
        let gun = context.add_slot_item(pid, 2, 0, 0).await.unwrap();
        context.set_slot_item(pid, material.api_id, 0, gun.api_id).await.unwrap();

        let row =
            slot_item::Entity::find_by_id(gun.api_id).one(context.db()).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        am.mst_id = ActiveValue::Set(MISSING_MST_ID);
        am.update(context.db()).await.unwrap();

        let result = context.powerup(pid, target.api_id, &[material.api_id], true).await;
        assert!(result.is_err(), "powerup gave {result:?}");
    }

    #[tokio::test]
    async fn an_incentive_for_an_unknown_ship_returns_an_error() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "incentive").await;

        context
            .add_incentive(
                pid,
                &[KcApiIncentiveItem {
                    api_mode: KcApiIncentiveMode::PreRegister as i64,
                    api_type: KcApiIncentiveType::Ship as i64,
                    api_mst_id: MISSING_MST_ID,
                    api_getmes: None,
                    api_slotitem_level: None,
                    amount: 1,
                    alv: 0,
                }],
            )
            .await
            .unwrap();

        let result = context.confirm_incentives(pid).await;
        assert!(result.is_err(), "confirm_incentives gave {result:?}");
    }

    #[tokio::test]
    async fn a_rival_with_an_unknown_rank_returns_an_error() {
        use emukc_internal::db::entity::profile::practice::rival;

        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "practice").await;

        let info = context.get_practice_rivals(pid).await.unwrap();
        // Drawn just now, so the next call loads the stored rivals instead of
        // drawing new ones.
        let mut cfg = info.cfg.into_active_model();
        cfg.last_generated = ActiveValue::Set(emukc_internal::time::chrono::Utc::now());
        cfg.update(context.db()).await.unwrap();

        let row =
            rival::Entity::find_by_id(info.rivals[0].id).one(context.db()).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        am.rank = ActiveValue::Set(99);
        am.update(context.db()).await.unwrap();

        let result = context.get_practice_rivals(pid).await;
        assert!(result.is_err(), "get_practice_rivals gave {result:?}");
    }
}
