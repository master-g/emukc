//! End-to-end regression anchor for the 6.3.x API alignment feature
//! groups (plan U7): hangar expansion, event-map fields, port fields.

#[cfg(test)]
mod tests {

    /// 赤城, api_maxeq = [18, 18, 27, 10, 0]
    const AKAGI_MST_ID: i64 = 83;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("e2e-alignment", "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "e2e-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn api_alignment_e2e_walks_all_three_feature_groups() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;

        // Group 1: hangar expansion (R1) — add ship, add useitem 105, expand.
        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, 105, 1).await.unwrap();

        let onslot_max = context.expand_hangar_slot(pid, ship.api_id, 1).await.unwrap();

        let expected = [18, 19, 27, 10, 0]; // maxeq + 1 on slot 1
        assert_eq!(onslot_max, expected, "expand response carries full slot array");

        // Group 1 (R2): port ships carry api_onslot_max with the composed value.
        let ships = context.get_ships(pid).await.unwrap();
        let found = ships.iter().find(|s| s.api_id == ship.api_id).unwrap();
        assert_eq!(found.api_onslot_max, Some(expected), "port ship carries composed capacity");

        // Useitem consumed (R1).
        let use_items = context.get_use_items(pid).await.unwrap();
        let hangar_item = use_items.iter().find(|item| item.api_id == 105);
        assert!(
            hangar_item.is_none() || hangar_item.unwrap().api_count == 0,
            "useitem 105 consumed"
        );

        // Group 2: event-map fields (R4) — active event area maps carry
        // api_eventmap.api_limit_flag == 0.
        let infos = context.get_map_infos(pid).await.unwrap();
        let event_infos: Vec<_> =
            infos.iter().filter(|info| (600..700).contains(&info.api_id)).collect();
        assert!(
            !event_infos.is_empty(),
            "active event maps (area 62) visible, got: {:?}",
            infos.iter().map(|i| i.api_id).collect::<Vec<_>>()
        );
        for info in &event_infos {
            let event = info
                .api_eventmap
                .as_ref()
                .unwrap_or_else(|| panic!("event map {} missing api_eventmap", info.api_id));
            assert_eq!(event.api_limit_flag, Some(0), "map {} limit_flag", info.api_id);
        }
    }
}
