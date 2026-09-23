//! The enemy formation belongs to the composition the route locked.
//!
//! A cell lists one formation per composition, and the formation used to come
//! from the first entry of that list whichever composition was fought. On 5-6,
//! and on 110 of the 1180 regular-map compositions, the two differ.

#[cfg(test)]
mod tests {
    /// Park a 6-ship sortie on a 5-6 day-battle cell whose compositions do not
    /// all share the first listed formation, with a differing composition
    /// already locked as the route leaves it. Returns the profile, the cell and
    /// that composition's formation.
    async fn parked_on_a_mixed_formation_cell(context: &crate::TestContext) -> (i64, i64, i64) {
        let account = context.sign_up("enemy-formation", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "enemy-formation").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        let mut fleet = [-1; 6];
        for slot in &mut fleet {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet).await.unwrap();

        let definition = context.codex().maps.map_definition(56).unwrap();
        let stage = definition.stage(&definition.default_variant).unwrap();
        let (cell_no, composition) = stage
            .cells
            .iter()
            .filter(|cell| cell.event_kind == 1 && matches!(cell.event_id, 4 | 5))
            .find_map(|cell| {
                let fleet = stage.enemy_fleets.get(&cell.cell_no)?;
                let first = fleet.formations.first().copied();
                let composition = fleet
                    .compositions
                    .iter()
                    .find(|c| c.formation.is_some() && c.formation != first)?;
                Some((cell.cell_no, composition.clone()))
            })
            .expect("5-6 has a cell whose compositions use different formations");
        let formation = composition.formation.unwrap();

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let store = context.sortie_store();
        let mut active = store.get_active(pid).unwrap();
        active.map_id = 56;
        active.stage_id = definition.default_variant.clone();
        active.current_cell_id = cell_no;
        active.boss_cell_id = stage.boss_cell_no;
        active.locked_enemy_composition = Some(composition);
        let _ = store.insert_active(pid, active);

        (pid, cell_no, formation)
    }

    #[tokio::test]
    async fn day_battle_enemy_formation_follows_the_locked_composition() {
        let context = crate::TestContext::new().await;
        let (pid, cell_no, formation) = parked_on_a_mixed_formation_cell(&context).await;

        let battle = context.sortie_battle(pid, 1).await.unwrap();
        assert_eq!(battle.api_formation[1], formation, "5-6 cell {cell_no}");
    }

    #[tokio::test]
    async fn night_start_enemy_formation_follows_the_locked_composition() {
        let context = crate::TestContext::new().await;
        let (pid, cell_no, formation) = parked_on_a_mixed_formation_cell(&context).await;

        let battle = context.sortie_sp_midnight_battle(pid, 1).await.unwrap();
        assert_eq!(battle.api_formation[1], formation, "5-6 cell {cell_no}");
    }
}
