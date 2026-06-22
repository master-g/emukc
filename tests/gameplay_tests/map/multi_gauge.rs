//! Tests for gauge index and map record field behavior
//!
//! Note: No regular maps have `gauge_count` > 1 in the current codex.
//! These tests verify initial map record state without going through
//! the full sortie flow (which is already covered by unlock.rs).

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-gauge", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "gauge-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn new_map_record_has_gauge_index_1() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let _infos = context.get_map_infos(pid).await.unwrap();

        let records = context.get_map_records(pid).await.unwrap();
        let record_11 = records.iter().find(|r| r.map_id == 11);
        assert!(record_11.is_some(), "map 1-1 should have a record");

        let record = record_11.unwrap();
        assert_eq!(record.gauge_index, 1, "new map record should start at gauge_index 1");
    }

    #[tokio::test]
    async fn map_record_cleared_field_starts_false() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let _infos = context.get_map_infos(pid).await.unwrap();
        let records = context.get_map_records(pid).await.unwrap();

        let record_11 = records.iter().find(|r| r.map_id == 11);
        assert!(record_11.is_some());
        assert!(!record_11.unwrap().cleared, "new map record should not be cleared");
    }

    #[tokio::test]
    async fn map_record_stage_id_defaults_to_empty() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let _infos = context.get_map_infos(pid).await.unwrap();
        let records = context.get_map_records(pid).await.unwrap();

        let record_11 = records.iter().find(|r| r.map_id == 11);
        assert!(record_11.is_some());
        assert_eq!(
            record_11.unwrap().stage_id,
            Some(String::new()),
            "new map record should have empty default stage_id"
        );
    }
}
