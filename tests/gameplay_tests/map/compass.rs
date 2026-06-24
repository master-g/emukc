//! Compass (羅針盤) regression tests.
//!
//! `api_rashin_flg` must key purely on whether the **departing** cell is a
//! physical branch node (out-degree > 1), not on the fleet-resolved candidate
//! count. The latter collapses a branch to its single forced target and so
//! skips the compass when advancing into a resource cell reached from a branch
//! node (the user-reported 2-1 symptom).
//!
//! The plan's oracle is map 2-1, but unlocking 2-1 in-test is heavy (it
//! requires clearing the 1-x cascade). Map 1-1 reproduces the exact bug
//! structure and is available to a fresh profile:
//!
//!   cell 0 → [1]      out-degree 1  (not a branch)
//!   cell 1 → [2, 3]   out-degree 2  (branch node)
//!   cell 2 → []       battle, dead-end
//!   cell 3 → []       boss, dead-end
//!
//! Advancing from branch node cell 1 into a child must spin the compass —
//! the same shape as advancing from 2-1's branch node cell 3 into resource
//! cell 2 or 5.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-compass", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "compass-tester").await.unwrap();
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
    async fn rashin_flg_keys_on_departing_branch_node() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        // start: departing = start cell 0 (out-degree 1) → no compass.
        let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        assert_eq!(start.cell_no, 1, "1-1 start deterministically lands on cell 1");
        assert!(!start.rashin_flg, "departing cell 0 (out-degree 1) must not spin the compass");
        assert_eq!(start.rashin_id, 0);

        // next: departing = branch node cell 1 (out-degree 2 → [2, 3]) → compass
        // spins regardless of which child routing picks. This is the
        // resource-cell-from-branch case that previously returned rashin_flg=0.
        let next = context.next_sortie(pid, None).await.unwrap();
        assert!(
            next.rashin_flg,
            "advancing from branch node cell 1 (out-degree 2) must spin the compass, got cell {}",
            next.cell_no
        );
        assert_eq!(next.rashin_id, 1);
        assert!(
            next.cell_no == 2 || next.cell_no == 3,
            "cell 1 routes only to its children 2/3, got {}",
            next.cell_no
        );
    }
}
