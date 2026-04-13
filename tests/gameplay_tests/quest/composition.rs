//! Quest system unit tests - validates bug fixes

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    #[test]
    fn test_composition_exact_match_requirement() {
        use emukc_internal::model::profile::fleet::Fleet;
        use emukc_internal::model::thirdparty::composition::*;

        let ship1 = ShipInstance {
            id: 1,
            mst_id: 1,
            level: 1,
        };
        let ship2 = ShipInstance {
            id: 2,
            mst_id: 2,
            level: 1,
        };
        let ships = vec![ship1, ship2];

        let mut fleet = Fleet::new(0, 1).unwrap();
        fleet.ships = [1, -1, -1, -1, -1, -1];

        let condition = Kc3rdQuestConditionComposition {
            groups: vec![Kc3rdQuestConditionShipGroup {
                ship: Kc3rdQuestConditionShip::Any,
                amount: Kc3rdQuestShipAmount {
                    min: 2,
                    max: 2,
                },
                lv: 0,
                position: 0,
                other_ships: false,
                white_list: None,
            }],
            fleet_id: 0,
            disallowed: None,
        };

        let codex = Codex::load(std::path::Path::new(".data/codex"), true).unwrap();

        // With 1 ship, should NOT satisfy requirement of exactly 2
        assert!(!validate_composition(&fleet, &ships, &condition, &codex));

        // With 2 ships, should satisfy
        fleet.ships = [1, 2, -1, -1, -1, -1];
        assert!(validate_composition(&fleet, &ships, &condition, &codex));
    }
}
