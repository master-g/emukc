//! Tests for the kaisou (equipment) operations driven by client form fields.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, username).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn slot_exchange_rejects_an_index_outside_the_five_slots() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "kaisou-range").await;
        let ship = context.add_ship(pid, 1).await.unwrap();

        for (src, dst) in [(0, 5), (5, 0), (-1, 0), (0, i64::MAX)] {
            let result = context.slot_exchange(ship.api_id, src, dst).await;
            assert!(
                matches!(result, Err(GameplayError::WrongType(_))),
                "({src}, {dst}) gave {result:?}"
            );
        }
        assert!(context.slot_exchange(ship.api_id, 0, 4).await.is_ok());
    }
}
