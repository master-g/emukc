//! Tests for 給糧艦 (間宮 / 伊良湖) use on a fleet: `api_req_member/itemuse_cond`.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    const MAMIYA: i64 = KcUseItemType::Mamiya as i64;
    const IRAKO: i64 = KcUseItemType::Irako as i64;

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, username).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn count(context: &crate::TestContext, pid: i64, item: i64) -> i64 {
        context.find_use_item(pid, item).await.map_or(0, |item| item.api_count)
    }

    /// A profile whose first fleet is one ship at condition 30.
    async fn lone_flagship(context: &crate::TestContext, username: &str) -> (i64, i64) {
        let pid = new_profile(context, username).await;
        let ship = context.add_ship(pid, 1).await.unwrap();
        context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
        let mut tired = context.find_ship(pid, ship.api_id).await.unwrap().unwrap();
        tired.api_cond = 30;
        context.update_ship(&tired).await.unwrap();
        (pid, ship.api_id)
    }

    #[tokio::test]
    async fn each_use_type_consumes_only_its_own_items() {
        let context = crate::TestContext::new().await;
        for (use_type, mamiya_used, irako_used) in [(1, 1, 0), (2, 0, 1), (3, 1, 1)] {
            let (pid, _) = lone_flagship(&context, &format!("cond-use-{use_type}")).await;
            context.add_use_item(pid, MAMIYA, 2).await.unwrap();
            context.add_use_item(pid, IRAKO, 2).await.unwrap();

            context.consume_cond_use_item(pid, 1, use_type).await.unwrap();

            assert_eq!(count(&context, pid, MAMIYA).await, 2 - mamiya_used, "use type {use_type}");
            assert_eq!(count(&context, pid, IRAKO).await, 2 - irako_used, "use type {use_type}");
        }
    }

    #[tokio::test]
    async fn mamiya_alone_needs_no_irako() {
        let context = crate::TestContext::new().await;
        let (pid, ship_id) = lone_flagship(&context, "cond-mamiya-only").await;
        context.add_use_item(pid, MAMIYA, 1).await.unwrap();

        context.consume_cond_use_item(pid, 1, 1).await.unwrap();

        assert_eq!(count(&context, pid, MAMIYA).await, 0);
        let ship = context.find_ship(pid, ship_id).await.unwrap().unwrap();
        assert_eq!(ship.api_cond, 40);
    }

    #[tokio::test]
    async fn irako_on_a_lone_flagship_raises_it() {
        let context = crate::TestContext::new().await;
        let (pid, ship_id) = lone_flagship(&context, "cond-irako-lone").await;
        // Each run draws 0 to 3 extra ships; enough runs reach a draw above 0.
        for _ in 0..20 {
            context.add_use_item(pid, IRAKO, 1).await.unwrap();
            context.consume_cond_use_item(pid, 1, 2).await.unwrap();
        }

        let ship = context.find_ship(pid, ship_id).await.unwrap().unwrap();
        assert_eq!(ship.api_cond, 100);
    }
}
