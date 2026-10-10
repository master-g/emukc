//! Aircraft shot down in a sortie stay down until resupply, and the aircraft
//! that flew gain proficiency (plan `2026-10-10-005`).

#[cfg(test)]
mod tests {
    use emukc_internal::crypto::rng;
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-proficiency", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "proficiency-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn carrier(context: &crate::TestContext, pid: i64) -> KcApiShip {
        context
            .get_ships(pid)
            .await
            .unwrap()
            .into_iter()
            .find(|ship| ship.api_ship_id == 83)
            .unwrap()
    }

    /// Fight the first battle of a 2-1 sortie and go home without passing port.
    /// Returns what each friendly ship had left in its slots when the battle ended.
    async fn one_battle(context: &crate::TestContext, pid: i64) -> Vec<[i64; 5]> {
        context.start_sortie(pid, 1, 2, 1).await.unwrap();
        context.sortie_battle(pid, 1).await.unwrap();
        let left = context
            .sortie_store()
            .get_pending_battle(pid)
            .unwrap()
            .friendly
            .iter()
            .map(|ship| ship.ship.api_onslot)
            .collect();
        context.sortie_battle_result(pid).await.unwrap();
        context.sortie_goback_port(pid).await.unwrap();
        left
    }

    #[tokio::test]
    async fn what_a_battle_leaves_in_the_slots_is_what_the_ship_keeps() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        // Six carriers with bombers: anti-air fire only reaches aircraft that strike.
        apply_scenario(&context, pid, &Scenario::carrier_cutin()).await.unwrap();
        rng::seed(1);

        let slots = |ships: Vec<KcApiShip>| {
            ships.into_iter().map(|ship| ship.api_onslot).collect::<Vec<_>>()
        };
        let before = slots(context.get_ships(pid).await.unwrap());
        let left = one_battle(&context, pid).await;
        let after = slots(context.get_ships(pid).await.unwrap());

        // The profile also owns its starter, who stayed home.
        assert_eq!(after[..left.len()], left, "every ship keeps what the battle left it");
        let total = |fleet: &[[i64; 5]]| fleet.iter().flatten().sum::<i64>();
        assert!(total(&after) < total(&before), "an air battle costs aircraft");
    }

    #[tokio::test]
    async fn fighters_gain_proficiency_and_show_it_once_back_in_port() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        apply_scenario(&context, pid, &Scenario::gunnery_cutin()).await.unwrap();
        rng::seed(1);

        // 零戦21型 gains at least 4 an air battle, so three pass level 1.
        for _ in 0..3 {
            one_battle(&context, pid).await;
        }
        let levels = |items: Vec<KcApiSlotItem>| {
            items
                .into_iter()
                .filter(|item| item.api_slotitem_id == 20)
                .map(|item| item.api_alv.unwrap_or(0))
                .collect::<Vec<_>>()
        };

        let at_sea = levels(context.get_slot_items(pid).await.unwrap());
        assert!(at_sea.iter().all(|level| *level == 0), "no level moves before port: {at_sea:?}");

        context.port_view(pid).await.unwrap();
        let in_port = levels(context.get_slot_items(pid).await.unwrap());
        assert!(in_port.iter().any(|level| *level >= 1), "a level shows in port: {in_port:?}");
    }

    #[tokio::test]
    async fn resupply_refills_every_slot_and_charges_bauxite_for_each_aircraft() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        apply_scenario(&context, pid, &Scenario::gunnery_cutin()).await.unwrap();

        let full = carrier(&context, pid).await;
        let mut short = full.clone();
        short.api_onslot[1] -= 3;
        short.api_onslot[3] -= 2;
        context.update_ship(&short).await.unwrap();
        let bauxite = context.get_materials(pid).await.unwrap().bauxite;

        context.charge_supply(pid, &[full.api_id], KcApiChargeKind::Plane, true).await.unwrap();

        assert_eq!(carrier(&context, pid).await.api_onslot, full.api_onslot);
        assert_eq!(context.get_materials(pid).await.unwrap().bauxite, bauxite - 25);
    }
}
