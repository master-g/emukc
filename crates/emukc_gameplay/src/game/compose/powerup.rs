use std::{
	collections::{BTreeMap, HashMap, HashSet},
	sync::LazyLock,
};

use emukc_db::{
	entity::profile::{fleet, item::slot_item, ship},
	sea_orm::entity::prelude::*,
};
use emukc_model::{
	codex::Codex,
	kc2::{KcApiShip, KcShipType},
	prelude::{ApiMstShip, ApiMstSlotitem, Kc3rdShip},
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::err::GameplayError;

/// (num of Maruyu, num of Maruyu Kai) -> success rate
static MARUYU_CHART: LazyLock<BTreeMap<(i64, i64), f64>> = LazyLock::new(|| {
	let mut map = BTreeMap::new();
	map.insert((1, 0), 1.0 / 2.0);
	map.insert((2, 0), 2.0 / 3.0);
	map.insert((3, 0), 3.0 / 4.0);
	map.insert((4, 0), 4.0 / 5.0);
	map.insert((5, 0), 1.0);

	map.insert((0, 1), 1.0 / 2.0);
	map.insert((1, 1), 2.0 / 3.0);
	map.insert((2, 1), 1.0);
	map.insert((3, 1), 5.0 / 6.0);
	map.insert((4, 1), 6.0 / 7.0);

	map.insert((0, 2), 3.0 / 4.0);
	map.insert((1, 2), 4.0 / 5.0);
	map.insert((2, 2), 5.0 / 6.0);
	map.insert((3, 2), 6.0 / 7.0);

	map.insert((0, 3), 4.0 / 5.0);
	map.insert((1, 3), 1.0);
	map.insert((2, 3), 7.0 / 8.0);

	map.insert((0, 4), 6.0 / 7.0);
	map.insert((1, 4), 7.0 / 8.0);

	map.insert((0, 5), 1.0);
	map
});

struct PowerUpResult {
	success: bool,
	ship: Option<ship::Model>,
	deck_ports: Option<Vec<fleet::Model>>,
	// slot item type[2] -> [slot item instance ID]
	unset_list: Option<BTreeMap<i64, Vec<i64>>>,
}

pub(crate) async fn powerup_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ship_id: i64,
	material_ships: &[i64],
	keep_slot_items: bool,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let mut result = PowerUpResult {
		success: false,
		ship: None,
		deck_ports: None,
		unset_list: None,
	};
	// find target ship

	let target_ship = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"No ship found for profile ID {} and ship ID {}",
			profile_id, ship_id
		))
	})?;

	// calculate powerup potentials

	let target_ship_mst = codex.find::<ApiMstShip>(&target_ship.mst_id)?;
	let target_ship_basic = codex.find::<Kc3rdShip>(&target_ship_mst.api_id)?;

	let target_api_ship: KcApiShip = target_ship.into();

	let kc3rd_ship = &target_ship_basic;

	let powerup_potentials =
		codex.cal_powerup_potentials(&target_api_ship, Some(target_ship_mst), Some(kc3rd_ship))?;

	// find material ships

	let material_ships = ship::Entity::find()
		.filter(ship::Column::Id.is_in(material_ships.to_owned()))
		.all(c)
		.await?;

	if material_ships.is_empty() {
		return Err(GameplayError::EntryNotFound(format!(
			"one or more material ships not found for profile ID {} and ship IDs {:?}",
			profile_id,
			material_ships.iter().map(|s| s.id).collect::<Vec<_>>()
		)));
	}

	// find the material ships' slot items
	let slot_item_ids: Vec<i64> = material_ships
		.iter()
		.flat_map(|s| vec![s.slot_1, s.slot_2, s.slot_3, s.slot_4, s.slot_5, s.slot_ex])
		.filter(|&sid| sid > 0)
		.collect();

	// check if slot items are kept
	if keep_slot_items {
		let slot_items = slot_item::Entity::find()
			.filter(slot_item::Column::Id.is_in(slot_item_ids.clone()))
			.all(c)
			.await?;
		let slot_item_mst_map: BTreeMap<i64, ApiMstSlotitem> =
			slot_items.iter().fold(BTreeMap::new(), |mut map, si| {
				let mst = codex.find::<ApiMstSlotitem>(&si.mst_id).unwrap();
				map.insert(si.id, mst.clone());
				map
			});
		// update slot items' equip_on to target ship
		slot_item::Entity::update_many()
			.col_expr(slot_item::Column::EquipOn, Expr::value(0))
			.filter(slot_item::Column::Id.is_in(slot_item_ids.clone()))
			.exec(c)
			.await?;

		// unset slot items
		result.unset_list = Some(
			slot_item_ids
				.iter()
				.map(|&sid| {
					let mst = slot_item_mst_map.get(&sid).unwrap();
					(mst.api_type[2], sid)
				})
				.fold(BTreeMap::new(), |mut map, (t, sid)| {
					map.entry(t).or_default().push(sid);
					map
				}),
		);
	} else {
		// power up without keeping slot items, will not turn these slot items into materials
		// so we can simply remove them
		slot_item::Entity::delete_many()
			.filter(slot_item::Column::Id.is_in(slot_item_ids))
			.exec(c)
			.await?;
	}

	// calculate powerup

	// [0]: firepower, [1]: torpedo, [2]: aa, [3]: armor
	let mut base_power_ups: [i64; 4] = material_ships
		.iter()
		.filter_map(|m| {
			let mst = codex.find::<ApiMstShip>(&m.mst_id).ok()?;
			mst.api_powup
		})
		.fold([0; 4], |mut acc, powup| {
			acc[0] += powup[0];
			acc[1] += powup[1];
			acc[2] += powup[2];
			acc[3] += powup[3];
			acc
		});

	let mut rng = SmallRng::from_entropy();
	for (i, v) in base_power_ups.iter_mut().enumerate() {
		if *v == 0 || powerup_potentials[i] == 0 {
			continue;
		}
		let vv = *v;
		*v = if rng.gen_bool(0.5) {
			result.success = true;
			vv + ((vv + 1) / 5)
		} else {
			(vv + (vv + 2) / 5) / 2
		};
	}

	// extra powerup, 0: Luck, 1: HP, 2: ASW
	let mut extra_power_ups: [i64; 3] = [0; 3];

	// Maruyu luck bonus
	{
		// collect all Maruyu from material ships
		let maruyu_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 163).cloned().collect();
		let maruyu_kai_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 402).cloned().collect();

		// calculate luck bonus
		if let Some(rate) =
			MARUYU_CHART.get(&(maruyu_ships.len() as i64, maruyu_kai_ships.len() as i64))
		{
			if rng.gen_bool(*rate) {
				extra_power_ups[0] = (maruyu_ships.len() as f64 * 1.2
					+ maruyu_kai_ships.len() as f64 * 1.6)
					.ceil() as i64;
			}
		}
	}

	// Mizuho/Kamoi HP bonus
	{
		// mizuho 瑞穂 451
		// mizuho kai 瑞穂改 348
		// kamoi 神威 162
		// kamoi kai 神威改 499
		// kamoi kai b 神威改母 500
		let mizuho_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 451).cloned().collect();
		let mizuho_kai_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 348).cloned().collect();
		let kamoi_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 162).cloned().collect();
		let kamoi_kai_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 499).cloned().collect();
		let kamoi_kai_b_ships: Vec<ship::Model> =
			material_ships.iter().filter(|m| m.mst_id == 500).cloned().collect();

		// mizuho/kamoi must be used in pairs
		let num_of_mizuho = mizuho_ships.len() + mizuho_kai_ships.len();
		if num_of_mizuho > 1 && [62, 72].contains(&target_ship_mst.api_ctype) {
			// mizuho can only be used to power up mizuho/kamoi
			let mut remaining_mizuho = mizuho_ships.len();
			let remaining_mizuho_kai = mizuho_kai_ships.len();
			let mut p = 0;

			// pair kai first
			let pairs_mizuho_kai = remaining_mizuho_kai / 2;
			p += pairs_mizuho_kai * 80;

			// on kai on base
			let pairs_mizuho_mizuho_kai = remaining_mizuho.min(remaining_mizuho_kai);
			p += pairs_mizuho_mizuho_kai * 70;
			remaining_mizuho -= pairs_mizuho_mizuho_kai;

			// pair base
			let pairs_mizuho = remaining_mizuho / 2;
			p += pairs_mizuho * 60;

			let mut bonus = p / 100 * 2;
			let remaining_p = p % 100;
			if p > 0 && rng.gen_bool(remaining_p as f64 / 100.0) {
				bonus += 2;
			}

			extra_power_ups[1] = bonus as i64;
		}

		let num_of_kamoi = kamoi_ships.len() + kamoi_kai_ships.len() + kamoi_kai_b_ships.len();
		if num_of_kamoi > 1 && [62, 72, 41, 37].contains(&target_ship_mst.api_ctype) {
			// kamoi can only be used to power up mizuho/kamoi and agano class, and yamato class
			let mut remaining_kamoi = kamoi_ships.len();
			let remaining_kamoi_kai = kamoi_kai_ships.len() + kamoi_kai_b_ships.len();
			let mut p = 0;

			// pair kai first
			let pairs_kamoi_kai = remaining_kamoi_kai / 2;
			p += pairs_kamoi_kai * 80;

			// on kai on base
			let pairs_kamoi_kamoi_kai = remaining_kamoi.min(remaining_kamoi_kai);
			p += pairs_kamoi_kamoi_kai * 70;
			remaining_kamoi -= pairs_kamoi_kamoi_kai;

			// pair base
			let pairs_kamoi = remaining_kamoi / 2;
			p += pairs_kamoi * 60;

			let mut bonus = p / 100 * 2;
			let remaining_p = p % 100;
			if p > 0 && rng.gen_bool(remaining_p as f64 / 100.0) {
				bonus += 2;
			}

			extra_power_ups[1] += bonus as i64;
		}
	}

	// DE bonus
	{
		let de_ships: Vec<(&ship::Model, &ApiMstShip)> = material_ships
			.iter()
			.filter_map(|s| {
				let mst = codex.find::<ApiMstShip>(&s.mst_id).ok()?;
				if mst.api_stype != KcShipType::DE as i64 {
					None
				} else {
					Some((s, mst))
				}
			})
			.collect();

		match de_ships.len() {
			0 => {
				// no DE
			}
			1 => {
				// single DE
				let chance_mod_luck = 32.0 + 0.7 * de_ships[0].0.level as f64;
				let chance_mod_aws = 100.0 - chance_mod_luck;
				if chance_mod_luck > 100.0 || rng.gen_bool(chance_mod_luck / 100.0) {
					extra_power_ups[0] += 1;
				} else if chance_mod_aws > 0.0 && rng.gen_bool(chance_mod_aws / 100.0) {
					extra_power_ups[2] += 1;
				}
			}
			_ => {}
		}
	}

	// apply power up

	todo!()
}

struct DeGroupParam {
	id: i64,
	mst_id: i64,
}

fn group_de_ships(codex: &Codex, params: &[DeGroupParam]) -> (Vec<(i64, i64)>, Vec<i64>) {
	let mut hp_pairs: Vec<(i64, i64)> = Vec::new();
	let mut rest: Vec<i64> = Vec::new();
	let mut paired_ids: HashSet<i64> = HashSet::new();

	let id_to_mst: HashMap<i64, i64> = params.iter().map(|p| (p.id, p.mst_id)).collect();

	for param in params {
		let current_id = param.id;
		let current_mst_id = param.mst_id;

		if paired_ids.contains(&current_id) {
			continue;
		}

		let Ok(related_ids) = codex.ships_before_and_after(current_id) else {
			rest.push(current_id);
			continue;
		};

		let mut pair_found = false;
		for &rel_id in &related_ids {
			if let Some(&rel_mst_id) = id_to_mst.get(&rel_id) {
				if rel_mst_id == current_mst_id
					&& rel_id != current_id
					&& !paired_ids.contains(&rel_id)
				{
					hp_pairs.push((current_id, rel_id));
					paired_ids.insert(current_id);
					paired_ids.insert(rel_id);
					pair_found = true;
					break;
				}
			}
		}

		if !pair_found {
			rest.push(current_id);
		}
	}

	(hp_pairs, rest)
}
