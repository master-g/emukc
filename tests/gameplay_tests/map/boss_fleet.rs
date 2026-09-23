//! Every regular map whose boss cell is a battle must carry enemy
//! compositions there, or `sortie_bosscomp` stays false for that map.
//!
//! 3-2's boss L had none until its K/L rows were scraped. 1-6 is the one
//! regular map without a boss battle: its goal N is an escort-success cell
//! (`event_id` 8), so no boss cell is a battle and it has nothing to carry.

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn every_regular_boss_battle_has_compositions() {
        let context = crate::TestContext::new().await;
        let maps = &context.codex().maps.maps;

        let mut missing = Vec::new();
        for (map_id, map) in maps.iter().filter(|(_, m)| !m.is_event) {
            for (key, variant) in &map.variants {
                let boss = variant.cells.iter().find(|c| c.cell_no == variant.boss_cell_no);
                if boss.is_none_or(|c| c.event_kind != 1) {
                    continue;
                }
                let has_comps = variant
                    .enemy_fleets
                    .get(&variant.boss_cell_no)
                    .is_some_and(|f| !f.compositions.is_empty());
                if !has_comps {
                    missing.push(format!("{map_id}/{key:?} cell {}", variant.boss_cell_no));
                }
            }
        }
        assert!(missing.is_empty(), "boss battles without compositions: {missing:?}");

        let kiss = &maps[&32].variants[""];
        assert!(kiss.enemy_fleets.contains_key(&kiss.boss_cell_no), "3-2 boss L must have a fleet");

        let escort = &maps[&16].variants[""];
        assert!(
            escort.cells.iter().all(|c| c.event_id != 5),
            "1-6 has no boss battle; its goal N is an escort-success cell"
        );
    }
}
