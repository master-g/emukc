//! Fleet composition validation for quests

use crate::{codex::Codex, prelude::ApiMstShip, profile::fleet::Fleet};

use super::{
	Kc3rdQuestConditionComposition, Kc3rdQuestConditionShip, Kc3rdQuestConditionShipGroup,
};

/// Ship instance (from database)
pub struct ShipInstance {
	pub id: i64,
	pub mst_id: i64,
	pub level: i64,
}

/// Validate if a fleet composition satisfies the quest condition
pub fn validate_composition(
	fleet: &Fleet,
	ships: &[ShipInstance],
	condition: &Kc3rdQuestConditionComposition,
	codex: &Codex,
) -> bool {
	// Check fleet_id matches
	if condition.fleet_id > 0 && fleet.id != condition.fleet_id {
		return false;
	}

	// Get ships in fleet
	let fleet_ships: Vec<&ShipInstance> = fleet
		.ships
		.iter()
		.filter_map(|&ship_id| {
			if ship_id > 0 {
				ships.iter().find(|s| s.id == ship_id)
			} else {
				None
			}
		})
		.collect();

	// Check disallowed ships
	if let Some(disallowed) = &condition.disallowed {
		for ship in &fleet_ships {
			for disallow_cond in disallowed {
				if matches_ship(ship, disallow_cond, codex) {
					return false;
				}
			}
		}
	}

	// Validate each ship group requirement
	for group in &condition.groups {
		if !validate_ship_group(&fleet_ships, group, codex) {
			return false;
		}
	}

	true
}

fn validate_ship_group(
	fleet_ships: &[&ShipInstance],
	group: &Kc3rdQuestConditionShipGroup,
	codex: &Codex,
) -> bool {
	let mut matched_count = 0;

	for (idx, ship) in fleet_ships.iter().enumerate() {
		// Check position requirement
		if group.position > 0 && (idx + 1) as i64 != group.position {
			continue;
		}

		// Check if ship matches the condition
		if !matches_ship(ship, &group.ship, codex) {
			continue;
		}

		// Check level requirement
		if ship.level < group.lv {
			continue;
		}

		// Check white list
		if let Some(white_list) = &group.white_list {
			if !white_list.contains(&ship.mst_id) {
				continue;
			}
		}

		matched_count += 1;
	}

	// Check amount requirement
	matched_count >= group.amount.min && matched_count <= group.amount.max
}

fn matches_ship(ship: &ShipInstance, condition: &Kc3rdQuestConditionShip, codex: &Codex) -> bool {
	match condition {
		Kc3rdQuestConditionShip::Any => true,
		Kc3rdQuestConditionShip::Ship(ids) => ids.contains(&ship.mst_id),
		Kc3rdQuestConditionShip::ShipType(types) => {
			if let Ok(mst) = codex.find::<ApiMstShip>(&ship.mst_id) {
				types.contains(&mst.api_stype)
			} else {
				false
			}
		}
		Kc3rdQuestConditionShip::ShipClass(classes) => {
			if let Ok(mst) = codex.find::<ApiMstShip>(&ship.mst_id) {
				classes.contains(&mst.api_ctype)
			} else {
				false
			}
		}
		_ => false, // Navy, HighSpeed, LowSpeed, Aviation, Carrier not implemented yet
	}
}
