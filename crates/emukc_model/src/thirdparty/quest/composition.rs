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
	if condition.fleet_id > 0 && fleet.index != condition.fleet_id {
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

	let indexed_fleet_ships: Vec<(usize, &ShipInstance)> =
		fleet_ships.iter().enumerate().map(|(idx, ship)| (idx + 1, *ship)).collect();
	let mut assigned_counts = vec![0; condition.groups.len()];

	if indexed_fleet_ships.is_empty() {
		return condition.groups.iter().all(|group| {
			let amount = effective_group_amount(group, 0);
			amount.min == 0
		});
	}

	validate_group_assignment(
		&indexed_fleet_ships,
		&condition.groups,
		codex,
		0,
		&mut assigned_counts,
	)
}

fn validate_group_assignment(
	fleet_ships: &[(usize, &ShipInstance)],
	groups: &[Kc3rdQuestConditionShipGroup],
	codex: &Codex,
	next_ship_idx: usize,
	assigned_counts: &mut [i64],
) -> bool {
	if next_ship_idx == fleet_ships.len() {
		return groups.iter().enumerate().all(|(group_idx, group)| {
			let amount = effective_group_amount(group, fleet_ships.len());
			let count = assigned_counts[group_idx];
			count >= amount.min && count <= amount.max
		});
	}

	for (group_idx, group) in groups.iter().enumerate() {
		let amount = effective_group_amount(group, fleet_ships.len());
		if assigned_counts[group_idx] >= amount.max {
			continue;
		}

		let (position, ship) = fleet_ships[next_ship_idx];
		if !matches_group(ship, position, group, codex) {
			continue;
		}

		assigned_counts[group_idx] += 1;
		if can_still_satisfy_groups(fleet_ships, groups, codex, next_ship_idx + 1, assigned_counts)
			&& validate_group_assignment(
				fleet_ships,
				groups,
				codex,
				next_ship_idx + 1,
				assigned_counts,
			) {
			return true;
		}
		assigned_counts[group_idx] -= 1;
	}

	false
}

fn can_still_satisfy_groups(
	fleet_ships: &[(usize, &ShipInstance)],
	groups: &[Kc3rdQuestConditionShipGroup],
	codex: &Codex,
	next_ship_idx: usize,
	assigned_counts: &[i64],
) -> bool {
	for (group_idx, group) in groups.iter().enumerate() {
		let amount = effective_group_amount(group, fleet_ships.len());
		let count = assigned_counts[group_idx];
		if count > amount.max {
			return false;
		}

		let remaining_matchable = fleet_ships[next_ship_idx..]
			.iter()
			.filter(|(position, ship)| matches_group(ship, *position, group, codex))
			.count() as i64;

		if count + remaining_matchable < amount.min {
			return false;
		}
	}

	true
}

fn effective_group_amount(
	group: &Kc3rdQuestConditionShipGroup,
	fleet_size: usize,
) -> super::Kc3rdQuestShipAmount {
	if group.position > 0 && group.amount.min == 0 && group.amount.max == 0 {
		return super::Kc3rdQuestShipAmount::exact(1);
	}

	if group.other_ships && group.position == 0 && group.amount.min == group.amount.max {
		return super::Kc3rdQuestShipAmount::range(group.amount.min, fleet_size as i64);
	}

	group.amount.clone()
}

fn matches_group(
	ship: &ShipInstance,
	position: usize,
	group: &Kc3rdQuestConditionShipGroup,
	codex: &Codex,
) -> bool {
	// Check position requirement
	if group.position > 0 && position as i64 != group.position {
		return false;
	}

	// Check if ship matches the condition
	if !matches_ship(ship, &group.ship, codex) {
		return false;
	}

	// Check level requirement
	if ship.level < group.lv {
		return false;
	}

	// Check white list
	if let Some(white_list) = &group.white_list
		&& !white_list.contains(&ship.mst_id)
	{
		return false;
	}

	true
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::thirdparty::{
		Kc3rdQuestConditionShip, Kc3rdQuestConditionShipGroup, Kc3rdQuestShipAmount,
	};

	fn ship(id: i64, mst_id: i64) -> ShipInstance {
		ShipInstance {
			id,
			mst_id,
			level: 1,
		}
	}

	fn fleet(id: i64, ships: &[i64]) -> Fleet {
		let mut fleet = Fleet::new(1, id).unwrap();
		let mut slots = [-1; 6];
		for (idx, ship_id) in ships.iter().enumerate() {
			slots[idx] = *ship_id;
		}
		fleet.ships = slots;
		fleet
	}

	#[test]
	fn wildcard_other_ships_group_treats_integer_amount_as_minimum() {
		let ships = vec![ship(1, 101), ship(2, 102), ship(3, 103)];
		let condition = Kc3rdQuestConditionComposition {
			groups: vec![Kc3rdQuestConditionShipGroup {
				ship: Kc3rdQuestConditionShip::Any,
				amount: Kc3rdQuestShipAmount::exact(2),
				lv: 0,
				position: 0,
				other_ships: true,
				white_list: None,
			}],
			disallowed: None,
			fleet_id: 1,
		};

		assert!(
			validate_composition(&fleet(1, &[1, 2, 3]), &ships, &condition, &Codex::default(),)
		);
		assert!(!validate_composition(&fleet(1, &[1]), &ships, &condition, &Codex::default(),));
	}

	#[test]
	fn composition_assignment_rejects_extra_unmatched_ships() {
		let ships = vec![ship(1, 34), ship(2, 35), ship(3, 36), ship(4, 37), ship(5, 99)];
		let condition = Kc3rdQuestConditionComposition {
			groups: vec![Kc3rdQuestConditionShipGroup {
				ship: Kc3rdQuestConditionShip::Ship(vec![34, 35, 36, 37]),
				amount: Kc3rdQuestShipAmount::exact(4),
				lv: 0,
				position: 0,
				other_ships: false,
				white_list: None,
			}],
			disallowed: None,
			fleet_id: 1,
		};

		assert!(validate_composition(
			&fleet(1, &[1, 2, 3, 4]),
			&ships,
			&condition,
			&Codex::default(),
		));
		assert!(!validate_composition(
			&fleet(1, &[1, 2, 3, 4, 5]),
			&ships,
			&condition,
			&Codex::default(),
		));
	}

	#[test]
	fn position_group_with_zero_amount_requires_the_flagship() {
		let ships = vec![ship(1, 11), ship(2, 21), ship(3, 22), ship(4, 23)];
		let condition = Kc3rdQuestConditionComposition {
			groups: vec![
				Kc3rdQuestConditionShipGroup {
					ship: Kc3rdQuestConditionShip::Ship(vec![11]),
					amount: Kc3rdQuestShipAmount::exact(0),
					lv: 0,
					position: 1,
					other_ships: false,
					white_list: None,
				},
				Kc3rdQuestConditionShipGroup {
					ship: Kc3rdQuestConditionShip::Ship(vec![21, 22, 23]),
					amount: Kc3rdQuestShipAmount::exact(3),
					lv: 0,
					position: 0,
					other_ships: false,
					white_list: None,
				},
			],
			disallowed: None,
			fleet_id: 1,
		};

		assert!(validate_composition(
			&fleet(1, &[1, 2, 3, 4]),
			&ships,
			&condition,
			&Codex::default(),
		));
		assert!(!validate_composition(
			&fleet(1, &[2, 1, 3, 4]),
			&ships,
			&condition,
			&Codex::default(),
		));
	}
}
