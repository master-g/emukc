//! Tests for `Ctx::port_view`.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        db::{
            entity::profile::quest,
            sea_orm::{ActiveModelTrait, ActiveValue},
        },
        prelude::Kc3rdQuestRequirement,
        time::chrono::Utc,
    };

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-port-view", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "port-view-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn insert_completed_quest(context: &crate::TestContext, profile_id: i64, quest_id: i64) {
        let quest_manifest = context.codex.quest.get(&quest_id).unwrap();
        let (requirements, requirement_type) = match &quest_manifest.requirements {
            Kc3rdQuestRequirement::And(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::And)
            }
            Kc3rdQuestRequirement::OneOf(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::OneOf)
            }
            Kc3rdQuestRequirement::Sequential(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::Sequential)
            }
        };

        quest::progress::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            quest_id: ActiveValue::Set(quest_id),
            status: ActiveValue::Set(quest::progress::Status::Activated),
            progress: ActiveValue::Set(quest::progress::Progress::Completed),
            period: ActiveValue::Set(quest_manifest.period.try_into().unwrap()),
            start_since: ActiveValue::Set(Utc::now()),
            requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
            requirement_type: ActiveValue::Set(requirement_type),
        }
        .insert(context.db.as_ref())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn quest_reward_claim_updates_materials_and_persists_slotitem_reward() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        let quest_id = 103;

        insert_completed_quest(&context, pid, quest_id).await;

        let before_materials = context.get_materials(pid).await.unwrap();
        let before_slotitems = context.get_slot_items(pid).await.unwrap();

        let reward_resp = context.quest_clear_and_claim_reward(pid, quest_id, None).await.unwrap();
        assert_eq!(reward_resp.api_material, [40, 40, 0, 40]);

        let after_materials = context.get_materials(pid).await.unwrap();
        let after_slotitems = context.get_slot_items(pid).await.unwrap();
        assert_eq!(after_materials.fuel, before_materials.fuel + 40);
        assert_eq!(after_materials.ammo, before_materials.ammo + 40);
        assert_eq!(after_materials.bauxite, before_materials.bauxite + 40);
        assert_eq!(after_slotitems.len(), before_slotitems.len() + 1);
        assert!(after_slotitems.iter().any(|item| item.api_slotitem_id == 42));

        let port = context.port_view(pid).await.unwrap();
        assert_eq!(port.materials.fuel, after_materials.fuel);
        assert_eq!(port.materials.ammo, after_materials.ammo);
        assert_eq!(port.materials.steel, after_materials.steel);
        assert_eq!(port.materials.bauxite, after_materials.bauxite);
    }

    /// Plan 002 AE7: returning to port drops the runtime state left behind by a
    /// mid-battle disconnect.
    #[tokio::test]
    async fn port_view_clears_pending_sortie_state() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;

        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        context.sortie_battle(pid, 1).await.unwrap();
        assert!(context.sortie_store.get_active(pid).is_some());

        context.port_view(pid).await.unwrap();

        assert!(context.sortie_store.get_active(pid).is_none());
        assert!(context.sortie_store.get_pending_result(pid).is_none());
    }
}
