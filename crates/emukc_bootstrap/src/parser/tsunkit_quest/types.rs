use emukc_model::prelude::*;

use super::{
	ClassId, CombatResult, List, Requirements, RequirementsCategory, RequirementsComp, Sortie,
};

impl ClassId {
	pub(super) fn to_kc3rd_ship_ids(&self, mst: &ApiManifest) -> Option<Kc3rdQuestConditionShip> {
		match self {
			ClassId::Integer(id) => {
				mst.find_ship(*id)?;
				Some(Kc3rdQuestConditionShip::Ship(*id))
			}
			ClassId::IntegerArray(ids) => {
				if ids.iter().any(|id| mst.find_ship(*id).is_none()) {
					return None;
				}
				Some(Kc3rdQuestConditionShip::Ships(ids.clone()))
			}
		}
	}

	pub(super) fn to_kc3rd_ship_class(&self, mst: &ApiManifest) -> Option<Kc3rdQuestConditionShip> {
		match self {
			ClassId::Integer(id) => {
				mst.find_ship_class(*id)?;
				Some(Kc3rdQuestConditionShip::ShipClass(*id))
			}
			ClassId::IntegerArray(ids) => {
				if ids.iter().any(|id| mst.find_ship_class(*id).is_none()) {
					return None;
				}
				Some(Kc3rdQuestConditionShip::ShipClasses(ids.clone()))
			}
		}
	}

	pub(super) fn to_kc3rd_ship_types(&self, mst: &ApiManifest) -> Option<Kc3rdQuestConditionShip> {
		match self {
			ClassId::Integer(id) => {
				mst.find_ship_type(*id)?;
				Some(Kc3rdQuestConditionShip::ShipType(*id))
			}
			ClassId::IntegerArray(ids) => {
				if ids.iter().any(|id| mst.find_ship_type(*id).is_none()) {
					return None;
				}
				Some(Kc3rdQuestConditionShip::ShipTypes(ids.clone()))
			}
		}
	}

	pub(super) fn find_ship_group(id: i64) -> Option<Kc3rdQuestConditionShip> {
		if id > 5000 && id < 6000 {
			return Some(Kc3rdQuestConditionShip::ShipType(id - 5000));
		}
		match id {
			1100 => Some(Kc3rdQuestConditionShip::HighSpeed),
			4001 => Some(Kc3rdQuestConditionShip::Navy(Kc3rdQuestShipNavy::USN)), // USN
			4002 => Some(Kc3rdQuestConditionShip::Navy(Kc3rdQuestShipNavy::RN)),  // RN
			// TODO: these two might not be correct, but since they always appear together, it's fine
			4004 => Some(Kc3rdQuestConditionShip::Navy(Kc3rdQuestShipNavy::RNN)), // RNN
			4005 => Some(Kc3rdQuestConditionShip::Navy(Kc3rdQuestShipNavy::RAN)), // RAN
			_ => None,
		}
	}

	fn find_ship_groups(ids: &[i64]) -> Option<Kc3rdQuestConditionShip> {
		if ids.iter().all(|id| *id > 5000 && *id < 6000) {
			Some(Kc3rdQuestConditionShip::ShipTypes(ids.iter().map(|id| id - 5000).collect()))
		} else if ids.iter().all(|id| *id > 1000 && *id < 2000) {
			// FIXME: this is a hack, but it's fine for now
			Some(Kc3rdQuestConditionShip::HighSpeed)
		} else {
			let navies: Vec<Kc3rdQuestShipNavy> = ids
				.iter()
				.filter_map(|id| match *id {
					4001 => Some(Kc3rdQuestShipNavy::USN),
					4002 => Some(Kc3rdQuestShipNavy::RN),
					4004 => Some(Kc3rdQuestShipNavy::RNN),
					4005 => Some(Kc3rdQuestShipNavy::RAN),
					_ => None,
				})
				.collect();
			if navies.is_empty() {
				return None;
			}
			Some(Kc3rdQuestConditionShip::Navies(navies))
		}
	}

	pub(super) fn to_kc3rd_ship_group_ships(&self) -> Option<Kc3rdQuestConditionShip> {
		match self {
			ClassId::Integer(id) => Self::find_ship_group(*id),
			ClassId::IntegerArray(ids) => Self::find_ship_groups(ids),
		}
	}

	pub(super) fn to_kc3rd_amount(&self) -> Kc3rdQuestShipAmount {
		match self {
			ClassId::Integer(id) => Kc3rdQuestShipAmount::Exactly(*id),
			ClassId::IntegerArray(ids) => {
				if ids.is_empty() {
					Kc3rdQuestShipAmount::Exactly(1)
				} else if ids.len() == 1 {
					Kc3rdQuestShipAmount::Exactly(ids[0])
				} else {
					Kc3rdQuestShipAmount::Range(ids[0], ids[1])
				}
			}
		}
	}
}

impl RequirementsComp {
	pub(super) fn to_kc3rd_ship_group(
		&self,
		mst: &ApiManifest,
	) -> Option<Kc3rdQuestConditionShipGroup> {
		let position = self.position.unwrap_or(0);
		let lv = if let Some(lv) = &self.lv {
			lv[0]
		} else {
			0
		};

		let ship = if let Some(ship_id) = &self.ship_id {
			ship_id.to_kc3rd_ship_ids(mst)?
		} else if let Some(class_id) = &self.class_id {
			class_id.to_kc3rd_ship_types(mst)?
		} else if let Some(family_id) = &self.family_id {
			family_id.to_kc3rd_ship_class(mst)?
		} else if let Some(group_id) = &self.group_id {
			group_id.to_kc3rd_ship_group_ships()?
		} else {
			Kc3rdQuestConditionShip::Any
		};

		let white_list: Option<Vec<i64>> = self.criteria.as_ref().map(|criteria| {
			criteria
				.group_id
				.iter()
				.filter_map(|group_id| match ClassId::find_ship_group(*group_id) {
					Some(Kc3rdQuestConditionShip::Ships(ids)) => Some(ids),
					_ => None,
				})
				.flat_map(|s| s.into_iter())
				.collect()
		});

		let amount = if let Some(amt) = &self.amount {
			amt.to_kc3rd_amount()
		} else {
			Kc3rdQuestShipAmount::Exactly(1)
		};

		Some(Kc3rdQuestConditionShipGroup {
			ship,
			amount,
			lv,
			position,
			white_list,
		})
	}
}

impl CombatResult {
	pub(super) fn to_kc3rd_combat_result(self) -> KcSortieResult {
		match self {
			CombatResult::Clear => KcSortieResult::Clear,
			CombatResult::S => KcSortieResult::Ranked(KcSortieResultRank::S),
			CombatResult::A => KcSortieResult::Ranked(KcSortieResultRank::A),
			CombatResult::B => KcSortieResult::Ranked(KcSortieResultRank::B),
			CombatResult::C => KcSortieResult::Ranked(KcSortieResultRank::C),
		}
	}
}

impl Sortie {
	fn parse_single_map_info(name: &str) -> Option<Kc3rdQuestConditionMapInfo> {
		let mut parts = name.splitn(3, '-');
		let area = parts.next()?.parse::<i64>().ok()?;
		let number = parts.next()?.parse::<i64>().ok()?;
		let phase =
			parts.next().and_then(|phase| phase.trim_start_matches('P').parse::<i64>().ok());

		Some(Kc3rdQuestConditionMapInfo {
			area,
			number,
			phase,
		})
	}

	pub(super) fn to_kc3rd_sortie(&self) -> Kc3rdQuestConditionSortie {
		let times = self.times.unwrap_or(1);
		let defeat_boss = self.boss.unwrap_or(false);
		let result = self.result.as_ref().map(|r| r.to_kc3rd_combat_result());

		let map = if let Some(maps) = &self.map {
			match maps {
				super::Id::String(id) => {
					Self::parse_single_map_info(id).map(Kc3rdQuestConditionSortieMap::One)
				}
				super::Id::StringArray(ids) => {
					let maps = ids
						.iter()
						.filter_map(|map| Self::parse_single_map_info(map))
						.collect::<Vec<_>>();

					if maps.is_empty() {
						None
					} else if maps.len() == 1 {
						Some(Kc3rdQuestConditionSortieMap::One(maps[0].clone()))
					} else if self.any.unwrap_or(false) {
						Some(Kc3rdQuestConditionSortieMap::AnyOf(maps))
					} else {
						Some(Kc3rdQuestConditionSortieMap::All(maps))
					}
				}
			}
		} else {
			None
		};

		Kc3rdQuestConditionSortie {
			composition: None,
			defeat_boss,
			fleet_id: 0,
			map,
			result,
			times,
		}
	}
}

impl From<List> for Requirements {
	fn from(list: List) -> Self {
		let category: RequirementsCategory = match list.category.as_str() {
			"conversion" => RequirementsCategory::Conversion,
			"equipexchange" => RequirementsCategory::Equipexchange,
			"exercise" => RequirementsCategory::Exercise,
			"expedition" => RequirementsCategory::Expedition,
			"fleet" => RequirementsCategory::Fleet,
			"modernization" => RequirementsCategory::Modernization,
			"scrapequipment" => RequirementsCategory::Scrapequipment,
			"simple" => RequirementsCategory::Simple,
			"sink" => RequirementsCategory::Sink,
			"sortie" => RequirementsCategory::Sortie,
			_ => {
				panic!("unknown category: {}", list.category);
			}
		};

		Self {
			category,
			comp: list.comp,
			fleet_id: None,
			disallowed: list.disallowed,
			comp_banned: None,
			sortie: list.sortie,
			subcategory: None,
			times: list.times,
			group_id: None,
			amount: list.amount,
			list: None,
			result: list.result,
			daily: list.daily,
			expeds: None,
			resources: list.resources,
			secretary: None,
			slots: list.slots,
			scrap: list.scrap,
			consume: list.consume,
			batch: None,
			secretary_banned: None,
			class_id: None,
			family_id: None,
		}
	}
}
