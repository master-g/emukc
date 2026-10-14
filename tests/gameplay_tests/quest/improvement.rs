//! Integration tests: an improvement attempt advances the slot-item
//! improvement quests (#110).
//!
//! - 619 装備の改修強化 — daily, `SlotItemImprovement`: 1
//! - 1167 装備の改修集中強化 — weekly, `SlotItemImprovement`: 3
//!
//! Every `remodel_slot` attempt counts, failed ones included (`KC3Kai` counts
//! every response regardless of `api_remodel_flag`); a level reset does not.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        crypto::rng,
        db::entity::profile::quest::progress::Progress,
        model::codex::remodel_slot::{encode_recipe_id, remodel_success_rate},
        prelude::*,
    };

    /// 明石: as flagship it opens the arsenal; its ★9 rate is below 100.
    const AKASHI: i64 = 182;
    /// 睦月, a second ship the 12cm単装砲 recipe accepts.
    const MUTSUKI: i64 = 1;
    /// 12cm単装砲; its only recipe is the variant one, which eats spare copies.
    const GUN_12CM: i64 = 1;
    const RECIPE: i64 = encode_recipe_id(GUN_12CM, 1);

    async fn arsenal_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-imp-{tag}"), "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "improve").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        let akashi = context.add_ship(pid, AKASHI).await.unwrap().api_id;
        let second = context.add_ship(pid, MUTSUKI).await.unwrap().api_id;
        context.update_fleet_ships(pid, 1, &[akashi, second, -1, -1, -1, -1]).await.unwrap();
        context
            .add_material(
                pid,
                &[
                    (MaterialCategory::Fuel, 10_000),
                    (MaterialCategory::Ammo, 10_000),
                    (MaterialCategory::Steel, 10_000),
                    (MaterialCategory::DevMat, 100),
                    (MaterialCategory::Screw, 100),
                ],
            )
            .await
            .unwrap();
        pid
    }

    /// A 12cm単装砲 at `stars`, plus the two spare copies any stage eats.
    async fn gun(context: &crate::TestContext, pid: i64, stars: i64) -> i64 {
        for _ in 0..2 {
            context.add_slot_item(pid, GUN_12CM, 0, 0).await.unwrap();
        }
        context.add_slot_item(pid, GUN_12CM, stars, 0).await.unwrap().api_id
    }

    async fn taken(context: &crate::TestContext, pid: i64, quest_id: i64) {
        context.quest_add(pid, quest_id).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
    }

    async fn progress(context: &crate::TestContext, pid: i64, quest_id: i64) -> Progress {
        let records = context.get_quest_records(pid).await.unwrap();
        records.iter().find(|r| r.quest_id == quest_id).expect("quest has a record").progress
    }

    /// Attempts the quest's `SlotItemImprovement` counter still asks for.
    /// (A debug-added idle quest reads `Completed`, so its progress says nothing.)
    async fn remaining(context: &crate::TestContext, pid: i64, quest_id: i64) -> i64 {
        let records = context.get_quest_records(pid).await.unwrap();
        let record = records.iter().find(|r| r.quest_id == quest_id).expect("quest has a record");
        let conds: Vec<Kc3rdQuestCondition> =
            serde_json::from_value(record.requirements.clone()).unwrap();
        match conds.as_slice() {
            [Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(n))] => {
                *n
            }
            other => panic!("unexpected conditions {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_successful_improvement_completes_619() {
        let context = crate::TestContext::new().await;
        let pid = arsenal_profile(&context, "ok").await;
        taken(&context, pid, 619).await;
        let target = gun(&context, pid, 0).await;

        let result = context.remodel_slot(pid, RECIPE, target, true).await.unwrap();

        assert!(result.success);
        assert_eq!(progress(&context, pid, 619).await, Progress::Completed);
    }

    #[tokio::test]
    async fn a_failed_improvement_completes_619_too() {
        let context = crate::TestContext::new().await;
        let pid = arsenal_profile(&context, "fail").await;
        taken(&context, pid, 619).await;
        let target = gun(&context, pid, 9).await;

        // The attempt's roll is the only draw `remodel_slot` makes, so find a
        // seed whose first draw misses the ★9 rate and replay it.
        let rate = remodel_success_rate(9, false);
        let seed = (0..)
            .find(|&s| {
                rng::seed(s);
                i64::from(rng::u32(0..100)) >= rate
            })
            .unwrap();
        rng::seed(seed);
        let result = context.remodel_slot(pid, RECIPE, target, false).await;
        rng::reseed_from_entropy();

        assert!(!result.unwrap().success, "seed {seed} must force a failure");
        assert_eq!(progress(&context, pid, 619).await, Progress::Completed);
    }

    #[tokio::test]
    async fn the_weekly_1167_needs_three_attempts() {
        let context = crate::TestContext::new().await;
        let pid = arsenal_profile(&context, "weekly").await;
        taken(&context, pid, 1167).await;
        let target = gun(&context, pid, 0).await;
        for _ in 0..2 {
            gun(&context, pid, 0).await;
        }

        for _ in 0..2 {
            context.remodel_slot(pid, RECIPE, target, true).await.unwrap();
        }
        assert_ne!(progress(&context, pid, 1167).await, Progress::Completed);

        context.remodel_slot(pid, RECIPE, target, true).await.unwrap();
        assert_eq!(progress(&context, pid, 1167).await, Progress::Completed);
    }

    #[tokio::test]
    async fn an_idle_619_is_not_advanced() {
        let context = crate::TestContext::new().await;
        let pid = arsenal_profile(&context, "idle").await;
        context.quest_add(pid, 619).await.unwrap();
        let target = gun(&context, pid, 0).await;

        context.remodel_slot(pid, RECIPE, target, true).await.unwrap();

        assert_eq!(remaining(&context, pid, 619).await, 1);
    }

    #[tokio::test]
    async fn a_level_reset_does_not_advance_619() {
        let context = crate::TestContext::new().await;
        let pid = arsenal_profile(&context, "recover").await;
        taken(&context, pid, 619).await;
        let target = gun(&context, pid, 5).await;
        context.add_use_item(pid, KcUseItemType::ArsenalResource as i64, 1).await.unwrap();

        context.remodel_slot_recover(pid, RECIPE, target, 3).await.unwrap();

        assert_eq!(remaining(&context, pid, 619).await, 1);
    }
}
