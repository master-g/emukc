//! Tests for listing the profiles of an account.

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn an_account_lists_only_its_own_profiles() {
        let context = crate::TestContext::new().await;
        let first = context.sign_up("first-account", "1234567").await.unwrap();
        context.new_profile(&first.access_token.token, "first").await.unwrap();
        let second = context.sign_up("second-account", "1234567").await.unwrap();
        let token = &second.access_token.token;
        assert!(context.account_profiles(token).await.unwrap().is_empty());

        let created = context.new_profile(token, "second").await.unwrap();
        let profiles = context.account_profiles(token).await.unwrap();

        let ids: Vec<i64> = profiles.iter().map(|profile| profile.id).collect();
        assert_eq!(ids, [created.profile.id]);
        context.start_game(token, ids[0]).await.unwrap();
    }
}
