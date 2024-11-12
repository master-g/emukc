use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{
	collections::{BTreeMap, HashMap, HashSet},
	sync::LazyLock,
};

use emukc_db::{
	entity::profile::{item::slot_item, ship},
	sea_orm::{entity::prelude::*, ActiveValue},
};
use emukc_model::{
	codex::{group::DeGroupParam, Codex},
	kc2::{KcApiShip, KcApiSlotItem, KcShipType},
	prelude::{ApiMstShip, ApiMstSlotitem, Kc3rdShip},
};

use crate::{err::GameplayError, game::slot_item::find_slot_items_by_id_impl};

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

pub struct PowerUpResult {
	pub success: bool,
	pub ship: Option<ship::Model>,
	// slot item type[2] -> [slot item instance ID]
	pub unset_slot_item_types: Option<HashSet<i64>>,
}

pub(crate) async fn powerup_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ship_id: i64,
	material_ships: &[i64],
	keep_slot_items: bool,
) -> Result<PowerUpResult, GameplayError>
where
	C: ConnectionTrait,
{
	let mut result = PowerUpResult {
		success: false,
		ship: None,
		unset_slot_item_types: None,
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

	let mut target_api_ship: KcApiShip = target_ship.into();

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
		result.unset_slot_item_types =
			Some(slot_item_mst_map.values().map(|mst| mst.api_type[2]).collect::<HashSet<i64>>());
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
	let mut extra_luck_powerup: i64 = 0;
	let mut extra_hp_powerup: i64 = 0;
	let mut extra_asw_powerup: i64 = 0;

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
				extra_luck_powerup += (maruyu_ships.len() as f64 * 1.2
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

			extra_hp_powerup += bonus as i64;
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

			extra_hp_powerup += bonus as i64;
		}
	}

	// DE bonus
	{
		let de_ships: Vec<DeGroupParam> = material_ships
			.iter()
			.filter_map(|s| {
				let mst = codex.find::<ApiMstShip>(&s.mst_id).ok()?;
				if mst.api_stype != KcShipType::DE as i64 {
					None
				} else {
					Some((s, mst))
				}
			})
			.map(|(s, mst)| DeGroupParam {
				id: s.id,
				mst_id: s.mst_id,
				ctype: mst.api_ctype,
			})
			.collect();

		let ship_id_model_lookup: HashMap<i64, &ship::Model> =
			material_ships.iter().map(|s| (s.id, s)).collect();

		let grouped = codex.group_de_ships(&de_ships);
		grouped.hp_pairs.iter().for_each(|(id1, id2)| {
			let lv1 = ship_id_model_lookup.get(id1).unwrap().level as f64;
			let lv2 = ship_id_model_lookup.get(id2).unwrap().level as f64;
			let hp_mod_rate = 26.0 + 0.35 * (lv1 + lv2);
			if hp_mod_rate > 100.0 || rng.gen_bool(hp_mod_rate / 100.0) {
				extra_hp_powerup += 1;
			} else {
				extra_luck_powerup += 1;
			}

			let asw_mod_rate = 10.0 + 0.40 * (lv1 + lv2);
			if asw_mod_rate > 100.0 || rng.gen_bool(asw_mod_rate / 100.0) {
				extra_asw_powerup += 1;
			}
			result.success = true;
		});
		grouped.other_pairs.iter().for_each(|(id1, id2)| {
			let lv1 = ship_id_model_lookup.get(id1).unwrap().level as f64;
			let lv2 = ship_id_model_lookup.get(id2).unwrap().level as f64;
			let asw_mod_rate = 10.0 + 0.40 * (lv1 + lv2);
			if asw_mod_rate > 100.0 || rng.gen_bool(asw_mod_rate / 100.0) {
				extra_asw_powerup += 1;
			} else {
				extra_luck_powerup += 1;
			}
			result.success = true;
		});
		grouped.rest.iter().for_each(|id| {
			let lv = ship_id_model_lookup.get(id).unwrap().level as f64;
			let luck_mod_rate = 32.0 + 0.7 * lv;
			if luck_mod_rate > 100.0 || rng.gen_bool(luck_mod_rate / 100.0) {
				extra_luck_powerup += 1;
			} else {
				extra_asw_powerup += 1;
			}
			result.success = true;
		});
	}

	// remove material ships

	for m in material_ships.iter() {
		ship::Entity::delete_by_id(m.id).exec(c).await?;
	}

	// apply power up
	let powerups = [
		base_power_ups[0],
		base_power_ups[1],
		base_power_ups[2],
		base_power_ups[3],
		extra_luck_powerup,
		extra_hp_powerup,
		extra_asw_powerup,
	];

	for (i, (add, cap)) in powerups.into_iter().zip(powerup_potentials).enumerate() {
		let bonus = add.min(cap);
		target_api_ship.api_kyouka[i] += bonus;
	}

	let target_ship_slot_items = find_slot_items_by_id_impl(
		c,
		&[
			target_ship.slot_1,
			target_ship.slot_2,
			target_ship.slot_3,
			target_ship.slot_4,
			target_ship.slot_5,
			target_ship.slot_ex,
		],
	)
	.await?;

	let target_ship_slot_items: Vec<KcApiSlotItem> =
		target_ship_slot_items.into_iter().map(std::convert::Into::into).collect();

	// recalculate ship status
	codex.cal_ship_status(&mut target_api_ship, &target_ship_slot_items)?;

	let mut am: ship::ActiveModel = target_api_ship.clone().into();

	am.id = ActiveValue::Unchanged(target_api_ship.api_id);
	am.profile_id = ActiveValue::Unchanged(profile_id);

	// update ship
	let m = am.update(c).await?;

	result.ship = Some(m);

	Ok(result)
}
