//! Integration tests: the equipment a quest takes at its claim
//! (plan 2026-10-09-002).
//!
//! - 641 「航空基地設営」事前準備 — scrap ドラム缶 x2, hold 7.7mm機銃 (37) x2
//!   and 九六式艦戦 (19) x2; rewards 設営隊 x2 and equipment 168.
//! - 614 機種転換 — a carrier flagship carrying 九七式艦攻(友永隊) (93), scrap
//!   two of equipment 17; the 93 becomes 94 and keeps its 改修 level.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    const DRUM: i64 = 75;
    const AKAGI: i64 = 83;

    async fn new_profile(context: &crate::TestContext, tag: &str) -> i64 {
        let account = context.sign_up(&format!("test-eq-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "equipment-quest").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn item(context: &crate::TestContext, pid: i64, mst_id: i64, stars: i64) -> i64 {
        context.add_slot_item(pid, mst_id, stars, 0).await.unwrap().api_id
    }

    /// Take the quest and fill its scrap counter with two of `scrapped`.
    async fn taken_with_scrap_done(
        context: &crate::TestContext,
        pid: i64,
        quest_id: i64,
        scrapped: i64,
    ) {
        context.quest_add(pid, quest_id).await.unwrap();
        context.quest_start(pid, quest_id).await.unwrap();
        let junk = [item(context, pid, scrapped, 0).await, item(context, pid, scrapped, 0).await];
        context.destroy_items(pid, &junk).await.unwrap();
    }

    /// The quest's `(api_state, invalid)` as the quest list shows it.
    async fn shown(context: &crate::TestContext, pid: i64, quest_id: i64) -> (i64, bool) {
        let view = context.quest_list_view(pid, 0).await.unwrap();
        let quest = view.items.iter().find(|q| q.no == quest_id).expect("quest is listed");
        (quest.state, quest.invalid)
    }

    async fn held(context: &crate::TestContext, pid: i64) -> Vec<KcApiSlotItem> {
        context.get_slot_items(pid).await.unwrap()
    }

    #[tokio::test]
    async fn prepared_equipment_is_needed_and_the_least_improved_goes() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "prepared").await;
        taken_with_scrap_done(&context, pid, 641, DRUM).await;

        assert_eq!(shown(&context, pid, 641).await, (2, false), "nothing to hand in yet");
        assert!(context.quest_clear_and_claim_reward(pid, 641, None).await.is_err());

        let improved_gun = item(&context, pid, 37, 3).await;
        item(&context, pid, 37, 0).await;
        item(&context, pid, 37, 0).await;
        item(&context, pid, 19, 0).await;
        let locked_fighter = item(&context, pid, 19, 0).await;
        context.toggle_slot_item_locked(pid, locked_fighter).await.unwrap();
        assert_eq!(shown(&context, pid, 641).await.0, 2, "a locked piece is not on offer");

        item(&context, pid, 19, 0).await;
        assert_eq!(shown(&context, pid, 641).await, (3, false));

        let scrap_refund = context.get_materials(pid).await.unwrap();
        context.quest_clear_and_claim_reward(pid, 641, None).await.unwrap();

        let left = held(&context, pid).await;
        let of = |mst: i64| left.iter().filter(|i| i.api_slotitem_id == mst).collect::<Vec<_>>();
        assert_eq!(of(37).iter().map(|i| i.api_id).collect::<Vec<_>>(), vec![improved_gun]);
        assert_eq!(of(19).iter().map(|i| i.api_id).collect::<Vec<_>>(), vec![locked_fighter]);
        assert_eq!(of(168).len(), 1, "the reward arrives");
        // Handing in is not scrapping: no refund comes with it.
        let after = context.get_materials(pid).await.unwrap();
        assert_eq!(after.steel, scrap_refund.steel);
        assert_eq!(after.ammo, scrap_refund.ammo);
    }

    #[tokio::test]
    async fn a_conversion_takes_the_flagship_piece_and_keeps_its_stars() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "conversion").await;
        taken_with_scrap_done(&context, pid, 614, 17).await;
        assert_eq!(shown(&context, pid, 614).await.0, 2, "no carrier carries it yet");

        let carrier = context.add_ship(pid, AKAGI).await.unwrap().api_id;
        context.update_fleet_ships(pid, 1, &[carrier, -1, -1, -1, -1, -1]).await.unwrap();
        let tomonaga = item(&context, pid, 93, 4).await;
        context.set_slot_item(carrier, 1, tomonaga).await.unwrap();
        context.toggle_slot_item_locked(pid, tomonaga).await.unwrap();

        // Locked: shown as done, flagged, and refused.
        assert_eq!(shown(&context, pid, 614).await, (3, true));
        assert!(context.quest_clear_and_claim_reward(pid, 614, None).await.is_err());
        assert!(held(&context, pid).await.iter().any(|i| i.api_id == tomonaga));

        context.toggle_slot_item_locked(pid, tomonaga).await.unwrap();
        assert_eq!(shown(&context, pid, 614).await, (3, false));
        let reward = context.quest_clear_and_claim_reward(pid, 614, None).await.unwrap();
        assert!(
            reward
                .api_bounus
                .iter()
                .filter_map(|b| b.api_item.as_ref())
                .any(|i| i.api_id == Some(94) && i.api_slotitem_level == Some(4)),
            "the answer names the kept level: {reward:?}"
        );

        let left = held(&context, pid).await;
        assert!(!left.iter().any(|i| i.api_slotitem_id == 93));
        let converted = left.iter().find(|i| i.api_slotitem_id == 94).expect("the new model");
        assert_eq!(converted.api_level, 4);
        let ship = context.find_ship(carrier).await.unwrap().unwrap();
        assert!(!ship.api_slot.contains(&tomonaga), "the ship no longer carries it");
    }
}
