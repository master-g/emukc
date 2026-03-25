//! Tests for quest event matching and `apply_event` logic

#[cfg(test)]
mod tests {
	use emukc_internal::prelude::{
		ExpeditionResult, Kc3rdQuestCondition, Kc3rdQuestConditionExpedition,
		Kc3rdQuestConditionFactory, Kc3rdQuestConditionScrap, QuestActionEvent,
	};

	// --- matches_event tests ---

	#[test]
	fn test_ship_construction_matches_ship_constructed_event() {
		let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3));
		let event = QuestActionEvent::ShipConstructed {
			ship_mst_id: 100,
			large: false,
		};
		assert!(cond.matches_event(&event));
	}

	#[test]
	fn test_ship_construction_does_not_match_slot_item_event() {
		let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3));
		let event = QuestActionEvent::SlotItemConstructed {
			item_mst_id: 50,
		};
		assert!(!cond.matches_event(&event));
	}

	#[test]
	fn test_scrap_ship_matches_ship_scrapped() {
		let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(2));
		let event = QuestActionEvent::ShipScrapped {
			ship_mst_id: 10,
		};
		assert!(cond.matches_event(&event));
	}

	#[test]
	fn test_scrap_equipment_matches_slot_item_scrapped() {
		let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(5));
		let event = QuestActionEvent::SlotItemScrapped {
			item_mst_id: 20,
		};
		assert!(cond.matches_event(&event));
	}

	#[test]
	fn test_repair_matches_ship_repaired() {
		let cond = Kc3rdQuestCondition::Repair(3);
		let event = QuestActionEvent::ShipRepaired {
			ship_id: 1,
		};
		assert!(cond.matches_event(&event));
	}

	#[test]
	fn test_resupply_matches_ship_resupplied() {
		let cond = Kc3rdQuestCondition::Resupply(2);
		let event = QuestActionEvent::ShipResupplied {
			ship_id: 1,
		};
		assert!(cond.matches_event(&event));
	}

	#[test]
	fn test_repair_does_not_match_resupply_event() {
		let cond = Kc3rdQuestCondition::Repair(3);
		let event = QuestActionEvent::ShipResupplied {
			ship_id: 1,
		};
		assert!(!cond.matches_event(&event));
	}

	#[test]
	fn test_expedition_matches_expedition_completed_event() {
		let cond = Kc3rdQuestCondition::Expedition(vec![Kc3rdQuestConditionExpedition {
			list: Some(vec!["37".to_string()]),
			times: 1,
		}]);
		let event = QuestActionEvent::ExpeditionCompleted {
			mission_id: 37,
			result: ExpeditionResult::Success,
			fleet_id: 1,
		};
		assert!(cond.matches_event(&event));
	}

	// --- apply_event tests ---

	#[test]
	fn test_apply_event_decrements_counter() {
		let mut cond =
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3));
		let event = QuestActionEvent::ShipConstructed {
			ship_mst_id: 100,
			large: false,
		};
		assert!(cond.apply_event(&event));
		assert_eq!(
			cond,
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(2))
		);
	}

	#[test]
	fn test_apply_event_does_not_go_below_zero() {
		let mut cond =
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(0));
		let event = QuestActionEvent::ShipConstructed {
			ship_mst_id: 100,
			large: false,
		};
		assert!(!cond.apply_event(&event));
		assert_eq!(
			cond,
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(0))
		);
	}

	#[test]
	fn test_apply_event_wrong_type_returns_false() {
		let mut cond =
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3));
		let event = QuestActionEvent::ShipRepaired {
			ship_id: 1,
		};
		assert!(!cond.apply_event(&event));
		// Counter unchanged
		assert_eq!(
			cond,
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3))
		);
	}

	#[test]
	fn test_apply_event_repair_decrements() {
		let mut cond = Kc3rdQuestCondition::Repair(5);
		let event = QuestActionEvent::ShipRepaired {
			ship_id: 42,
		};
		assert!(cond.apply_event(&event));
		assert_eq!(cond, Kc3rdQuestCondition::Repair(4));
	}

	#[test]
	fn test_apply_event_resupply_decrements() {
		let mut cond = Kc3rdQuestCondition::Resupply(1);
		let event = QuestActionEvent::ShipResupplied {
			ship_id: 7,
		};
		assert!(cond.apply_event(&event));
		assert_eq!(cond, Kc3rdQuestCondition::Resupply(0));
		// Now at zero, should not decrement further
		assert!(!cond.apply_event(&event));
	}

	#[test]
	fn test_apply_event_scrap_ship_decrements() {
		let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(2));
		let event = QuestActionEvent::ShipScrapped {
			ship_mst_id: 10,
		};
		assert!(cond.apply_event(&event));
		assert_eq!(cond, Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(1)));
	}

	#[test]
	fn test_apply_event_scrap_equipment_decrements() {
		let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(3));
		let event = QuestActionEvent::SlotItemScrapped {
			item_mst_id: 20,
		};
		assert!(cond.apply_event(&event));
		assert_eq!(cond, Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(2)));
	}

	#[test]
	fn test_apply_multiple_events_to_completion() {
		let mut cond = Kc3rdQuestCondition::Repair(2);
		let event = QuestActionEvent::ShipRepaired {
			ship_id: 1,
		};

		assert!(cond.apply_event(&event)); // 2 -> 1
		assert!(!cond.is_satisfied());

		assert!(cond.apply_event(&event)); // 1 -> 0
		assert!(cond.is_satisfied());

		assert!(!cond.apply_event(&event)); // already 0
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_apply_expedition_event_decrements_matching_condition() {
		let mut cond = Kc3rdQuestCondition::Expedition(vec![
			Kc3rdQuestConditionExpedition {
				list: Some(vec!["37".to_string(), "38".to_string()]),
				times: 2,
			},
			Kc3rdQuestConditionExpedition {
				list: None,
				times: 3,
			},
		]);
		let event = QuestActionEvent::ExpeditionCompleted {
			mission_id: 37,
			result: ExpeditionResult::Success,
			fleet_id: 1,
		};

		assert!(cond.apply_event(&event));
		assert_eq!(
			cond,
			Kc3rdQuestCondition::Expedition(vec![
				Kc3rdQuestConditionExpedition {
					list: Some(vec!["37".to_string(), "38".to_string()]),
					times: 1,
				},
				Kc3rdQuestConditionExpedition {
					list: None,
					times: 3,
				},
			])
		);
	}
}
