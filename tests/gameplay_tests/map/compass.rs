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

    /// Data invariant that keeps the compass rule honest across every map.
    ///
    /// `rashin_flg` keys on `next_cells.len() > 1`, while `has_next` keys on
    /// `next_cells` OR `routing_rules` (see `cell_has_routing_outgoing`). These
    /// two definitions of "out-degree" only stay consistent as long as
    /// `next_cells` is a superset of every routable target. If a future
    /// data-source sync produced a cell whose `routing_rules` could reach a
    /// branch target absent from `next_cells`, `next_cells` would under-count
    /// the branch and the compass would wrongly skip at a 分岐点 — the exact bug
    /// this fix closed, re-introduced through data drift instead of code.
    ///
    /// Assert the structural guard: for every cell, each `routing_rules` target
    /// is listed in that cell's `next_cells`. Currently 0 violations across all
    /// 676 cells; this fires the moment that stops being true.
    #[tokio::test]
    async fn routing_targets_are_subset_of_next_cells() {
        let context = new_context().await;
        let catalog = &context.codex().maps;

        let mut violations = Vec::new();
        for (map_id, def) in &catalog.maps {
            for (vkey, variant) in &def.variants {
                let next_by_cell: std::collections::BTreeMap<i64, std::collections::BTreeSet<i64>> =
                    variant
                        .cells
                        .iter()
                        .map(|c| (c.cell_no, c.next_cells.iter().copied().collect()))
                        .collect();
                for (&from_cell, rules) in &variant.routing_rules {
                    let Some(next) = next_by_cell.get(&from_cell) else {
                        violations.push(format!(
                            "map {map_id} variant {vkey:?}: routing_rules key cell {from_cell} \
                             has no matching cell"
                        ));
                        continue;
                    };
                    for rule in rules {
                        if !next.contains(&rule.to_cell_no) {
                            violations.push(format!(
                                "map {map_id} variant {vkey:?} cell {from_cell}: routing target {} \
                                 absent from next_cells {next:?}",
                                rule.to_cell_no
                            ));
                        }
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "routing targets must be a subset of next_cells so rashin_flg never \
             under-counts a branch ({} violations):\n{}",
            violations.len(),
            violations.join("\n")
        );
    }
}
