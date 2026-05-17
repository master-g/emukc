//! Tests for sortie retreat (goback_port) behavior

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-retreat", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "retreat-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn setup_fleet(context: &crate::TestContext, pid: i64) {
        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
    }

    #[tokio::test]
    async fn goback_port_after_starting_sortie_clears_state() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        assert_eq!(start.maparea_id, 1);

        let result = context.sortie_goback_port(pid).await;
        assert!(result.is_ok(), "goback_port should succeed after starting sortie");

        let records = context.get_map_records(pid).await.unwrap();
        let rec = records.iter().find(|r| r.map_id == 11).expect("map 1-1 record should exist");
        assert!(!rec.cleared, "map should not be cleared after retreating without battle");

        let second = context.sortie_goback_port(pid).await;
        assert!(second.is_err(), "goback_port without active sortie should error");
    }

    #[tokio::test]
    async fn goback_port_after_battle_clears_state() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let mut current_cell = start.cell_no;
        let boss_cell = start.boss_cell_no;
        let mut boss_killed = false;

        loop {
            let battle = context.sortie_battle(pid, 1).await;
            if battle.is_err() {
                break;
            }
            let result = context.sortie_battle_result(pid).await;
            if result.is_err() {
                break;
            }

            if current_cell == boss_cell {
                boss_killed = true;
                break;
            }

            match context.next_sortie(pid, None).await {
                Ok(next) => current_cell = next.cell_no,
                Err(_) => break,
            }
        }

        let _ = context.sortie_goback_port(pid).await;

        if !boss_killed {
            let records = context.get_map_records(pid).await.unwrap();
            let rec = records.iter().find(|r| r.map_id == 11);
            if let Some(rec) = rec {
                assert!(
                    !rec.cleared,
                    "map should not be marked cleared after goback without boss kill"
                );
            }
        }

        let second = context.sortie_goback_port(pid).await;
        assert!(second.is_err(), "goback_port without active sortie should error");
    }

    #[tokio::test]
    async fn goback_port_without_active_sortie_errors() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let result = context.sortie_goback_port(pid).await;
        assert!(result.is_err(), "goback_port without active sortie should error");
    }
}
