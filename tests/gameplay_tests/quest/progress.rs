//! Tests for quest progress calculation and `is_satisfied` logic

#[cfg(test)]
mod tests {
	use emukc_internal::model::profile::quest::QuestProgressStatus;
	use emukc_internal::prelude::{
		Kc3rdQuestCondition, Kc3rdQuestConditionComposition, Kc3rdQuestConditionConsumption,
		Kc3rdQuestConditionExercise, Kc3rdQuestConditionExpedition, Kc3rdQuestConditionFactory,
		Kc3rdQuestConditionMaterialConsumption, Kc3rdQuestConditionModelConversion,
		Kc3rdQuestConditionModernization, Kc3rdQuestConditionScrap, Kc3rdQuestConditionShip,
		Kc3rdQuestConditionSortie, Kc3rdQuestRequirement, KcSortieResult,
	};

	// --- is_satisfied tests ---

	#[test]
	fn test_factory_ship_construction_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(0));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_factory_ship_construction_not_satisfied_when_nonzero() {
		let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(3));
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_factory_slotitem_construction_satisfied_when_zero() {
		let cond =
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(0));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_factory_slotitem_improvement_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(0));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_scrap_any_equipment_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(0));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_scrap_any_ship_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(0));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_scrap_specific_items_never_satisfied() {
		let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![]));
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_repair_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Repair(0);
		assert!(cond.is_satisfied());
		assert!(!Kc3rdQuestCondition::Repair(1).is_satisfied());
	}

	#[test]
	fn test_resupply_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Resupply(0);
		assert!(cond.is_satisfied());
		assert!(!Kc3rdQuestCondition::Resupply(5).is_satisfied());
	}

	#[test]
	fn test_sortie_satisfied_when_zero_times() {
		let cond = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
			composition: None,
			defeat_boss: false,
			fleet_id: 0,
			map: None,
			result: None,
			times: 0,
		});
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_sortie_not_satisfied_when_nonzero_times() {
		let cond = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
			composition: None,
			defeat_boss: true,
			fleet_id: 1,
			map: None,
			result: Some(KcSortieResult::Any),
			times: 3,
		});
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_exercise_satisfied_when_zero_times() {
		let cond = Kc3rdQuestCondition::Exercise(Kc3rdQuestConditionExercise {
			times: 0,
			expect_result: KcSortieResult::Any,
			expire_next_day: false,
			groups: None,
		});
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_exercise_not_satisfied_when_nonzero() {
		let cond = Kc3rdQuestCondition::Exercise(Kc3rdQuestConditionExercise {
			times: 5,
			expect_result: KcSortieResult::Clear,
			expire_next_day: false,
			groups: None,
		});
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_expedition_satisfied_when_all_zero() {
		let cond = Kc3rdQuestCondition::Expedition(vec![
			Kc3rdQuestConditionExpedition {
				list: None,
				times: 0,
			},
			Kc3rdQuestConditionExpedition {
				list: Some(vec!["A1".to_string()]),
				times: 0,
			},
		]);
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_expedition_not_satisfied_when_any_nonzero() {
		let cond = Kc3rdQuestCondition::Expedition(vec![
			Kc3rdQuestConditionExpedition {
				list: None,
				times: 0,
			},
			Kc3rdQuestConditionExpedition {
				list: None,
				times: 1,
			},
		]);
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_sink_satisfied_when_zero() {
		let cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::Any, 0);
		assert!(cond.is_satisfied());
		assert!(!Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::Any, 2).is_satisfied());
	}

	#[test]
	fn test_modernization_satisfied_when_zero_times() {
		let cond = Kc3rdQuestCondition::Modernization(Kc3rdQuestConditionModernization {
			target_ship: Kc3rdQuestConditionShip::Any,
			material_ship: Kc3rdQuestConditionShip::Any,
			batch_size: 1,
			times: 0,
		});
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_modernization_not_satisfied_when_nonzero() {
		let cond = Kc3rdQuestCondition::Modernization(Kc3rdQuestConditionModernization {
			target_ship: Kc3rdQuestConditionShip::Any,
			material_ship: Kc3rdQuestConditionShip::Any,
			batch_size: 2,
			times: 3,
		});
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_composition_never_satisfied_via_is_satisfied() {
		// Composition is validated separately via fleet check
		let cond = Kc3rdQuestCondition::Composition(Kc3rdQuestConditionComposition {
			groups: vec![],
			fleet_id: 0,
			disallowed: None,
		});
		assert!(!cond.is_satisfied());
	}

	#[test]
	fn test_consumption_always_satisfied() {
		let cond = Kc3rdQuestCondition::Consumption(Kc3rdQuestConditionConsumption::Resources(
			Kc3rdQuestConditionMaterialConsumption {
				fuel: 100,
				ammo: 0,
				steel: 0,
				bauxite: 0,
			},
		));
		assert!(cond.is_satisfied());
	}

	#[test]
	fn test_model_conversion_always_satisfied() {
		let cond = Kc3rdQuestCondition::ModelConversion(Kc3rdQuestConditionModelConversion {
			secretary: None,
			banned_secretary: None,
			slots: None,
		});
		assert!(cond.is_satisfied());
	}

	// --- calculate_progress tests ---

	#[test]
	fn test_and_progress_all_complete() {
		let req = Kc3rdQuestRequirement::And(vec![
			Kc3rdQuestCondition::Repair(0),
			Kc3rdQuestCondition::Resupply(0),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Completed);
	}

	#[test]
	fn test_and_progress_half_complete() {
		let req = Kc3rdQuestRequirement::And(vec![
			Kc3rdQuestCondition::Repair(0),
			Kc3rdQuestCondition::Resupply(5),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Half);
	}

	#[test]
	fn test_and_progress_none_complete() {
		let req = Kc3rdQuestRequirement::And(vec![
			Kc3rdQuestCondition::Repair(3),
			Kc3rdQuestCondition::Resupply(5),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Empty);
	}

	#[test]
	fn test_and_progress_80_percent() {
		// 4 out of 5 = 80%
		let req = Kc3rdQuestRequirement::And(vec![
			Kc3rdQuestCondition::Repair(0),
			Kc3rdQuestCondition::Resupply(0),
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(0)),
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(0)),
			Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(1)),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Eighty);
	}

	#[test]
	fn test_oneof_progress_any_satisfied() {
		let req = Kc3rdQuestRequirement::OneOf(vec![
			Kc3rdQuestCondition::Repair(5),
			Kc3rdQuestCondition::Resupply(0), // satisfied
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Completed);
	}

	#[test]
	fn test_oneof_progress_none_satisfied() {
		let req = Kc3rdQuestRequirement::OneOf(vec![
			Kc3rdQuestCondition::Repair(5),
			Kc3rdQuestCondition::Resupply(3),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Empty);
	}

	#[test]
	fn test_sequential_progress_all_complete() {
		let req = Kc3rdQuestRequirement::Sequential(vec![
			Kc3rdQuestCondition::Repair(0),
			Kc3rdQuestCondition::Resupply(0),
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Completed);
	}

	#[test]
	fn test_sequential_progress_first_incomplete() {
		let req = Kc3rdQuestRequirement::Sequential(vec![
			Kc3rdQuestCondition::Repair(3), // not done
			Kc3rdQuestCondition::Resupply(0),
		]);
		// 0 out of 2 complete
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Empty);
	}

	#[test]
	fn test_sequential_progress_second_incomplete() {
		let req = Kc3rdQuestRequirement::Sequential(vec![
			Kc3rdQuestCondition::Repair(0),   // done
			Kc3rdQuestCondition::Resupply(3), // not done
		]);
		// 1 out of 2 = 50%
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Half);
	}

	#[test]
	fn test_and_progress_with_sortie_condition() {
		// Regression test: sortie conditions were always returning false in is_satisfied
		let req = Kc3rdQuestRequirement::And(vec![Kc3rdQuestCondition::Sortie(
			Kc3rdQuestConditionSortie {
				composition: None,
				defeat_boss: true,
				fleet_id: 1,
				map: None,
				result: Some(KcSortieResult::Any),
				times: 0, // completed
			},
		)]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Completed);
	}

	#[test]
	fn test_and_progress_mixed_sortie_and_factory() {
		let req = Kc3rdQuestRequirement::And(vec![
			Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
				composition: None,
				defeat_boss: false,
				fleet_id: 0,
				map: None,
				result: None,
				times: 0, // done
			}),
			Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(2)), // not done
		]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Half);
	}

	#[test]
	fn test_empty_conditions_is_completed() {
		let req = Kc3rdQuestRequirement::And(vec![]);
		assert_eq!(req.calculate_progress(&req), QuestProgressStatus::Completed);
	}
}
