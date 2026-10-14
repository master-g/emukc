//! Integration tests: claiming a quest grants its furniture, war result and
//! expedition-supply rewards (#113).
//!
//! - 816 艦隊、三周年！ — furniture
//! - 854 戦果拡張任務！「Z作戦」前段作戦 — 戦果
//! - 427 遠征「補給」支援体制を強化せよ！ — expedition supply
//! - 146 改修工廠を準備せよ！ — the improvement factory, not gated here
//!
//! The issue names 246 and 284; both also ask for a fleet and sortie counts the
//! debug `quest_add` cannot fake, so these sortie-only quests stand in for them.
//! The rewards go through the same arms.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-cr-{tag}"), "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "claim").await.unwrap();
        context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id
    }

    /// Hand the profile the quest fulfilled and claim it with the first choice
    /// of each choice group.
    async fn claim(context: &crate::TestContext, pid: i64, quest_id: i64) {
        let choices = crate::CODEX.quest[&quest_id].choice_rewards.iter().map(|_| 0).collect();
        context.quest_add(pid, quest_id).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
        context.quest_clear_and_claim_reward(pid, quest_id, Some(choices)).await.unwrap();
    }

    async fn furniture(context: &crate::TestContext, pid: i64) -> Vec<i64> {
        context.get_furnitures(pid).await.unwrap().iter().map(|f| f.api_id).collect()
    }

    /// The furniture 816 rewards.
    fn reward_furniture() -> i64 {
        crate::CODEX.quest[&816]
            .additional_rewards
            .iter()
            .find(|r| r.category == Kc3rdQuestRewardCategory::Furniture)
            .unwrap()
            .api_id
    }

    #[tokio::test]
    async fn a_furniture_reward_is_added() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "furniture").await;
        assert!(!furniture(&context, pid).await.contains(&reward_furniture()));

        claim(&context, pid, 816).await;

        assert!(furniture(&context, pid).await.contains(&reward_furniture()));
    }

    #[tokio::test]
    async fn furniture_already_owned_does_not_fail_the_claim() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "owned").await;
        context.add_furniture(pid, reward_furniture()).await.unwrap();

        claim(&context, pid, 816).await;

        let owned = furniture(&context, pid).await;
        assert_eq!(owned.iter().filter(|&&id| id == reward_furniture()).count(), 1);
    }

    #[tokio::test]
    async fn a_war_result_reward_is_added() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "war").await;
        let before = context.get_user_basic(pid).await.unwrap().1.api_war_result;

        claim(&context, pid, 854).await;

        let amount = crate::CODEX.quest[&854]
            .additional_rewards
            .iter()
            .find(|r| r.category == Kc3rdQuestRewardCategory::WarResult)
            .unwrap()
            .amount;
        let after = context.get_user_basic(pid).await.unwrap().1.api_war_result;
        assert_eq!(after - before, amount);
    }

    #[tokio::test]
    async fn the_expedition_supply_reward_turns_it_on() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "supply").await;
        // 427 consumes 800 steel.
        context.add_material(pid, &[(MaterialCategory::Steel, 1000)]).await.unwrap();
        assert_eq!(context.get_user_basic(pid).await.unwrap().1.api_extra_supply[0], 0);

        claim(&context, pid, 427).await;

        assert_eq!(context.get_user_basic(pid).await.unwrap().1.api_extra_supply[0], 1);
    }

    #[tokio::test]
    async fn the_factory_reward_claims_and_changes_nothing_else() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "factory").await;
        // 146 asks for 明石 as flagship.
        let akashi = context.add_ship(pid, 182).await.unwrap().api_id;
        context.update_fleet_ships(pid, 1, &[akashi, -1, -1, -1, -1, -1]).await.unwrap();
        let flags = |basic: KcApiUserBasic| {
            (basic.api_large_dock, basic.api_extra_supply, basic.api_war_result)
        };
        let before = flags(context.get_user_basic(pid).await.unwrap().1);

        claim(&context, pid, 146).await;

        assert_eq!(flags(context.get_user_basic(pid).await.unwrap().1), before);
    }
}
