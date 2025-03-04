use emukc_db::{
	entity::profile::ship,
	sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcApiChargeKind, KcApiChargeResp, KcApiChargeShip, MaterialCategory},
	prelude::ApiMstShip,
};

use crate::{err::GameplayError, game::material::deduct_material_impl};

pub(crate) async fn supply_fleet_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ship_ids: &[i64],
	mode: KcApiChargeKind,
	supply_aircrafts: bool,
) -> Result<KcApiChargeResp, GameplayError>
where
	C: ConnectionTrait,
{
	let ships = ship::Entity::find()
		.filter(ship::Column::ProfileId.eq(profile_id))
		.filter(ship::Column::Id.is_in(ship_ids.to_owned()))
		.all(c)
		.await?;

	if ships.is_empty() {
		return Err(GameplayError::EntryNotFound(format!(
			"No ships found for profile ID {} and ship IDs {:?}",
			profile_id, ship_ids
		)));
	}

	let mut resp = KcApiChargeResp {
		api_ship: vec![],
		api_material: [0; 4],
		api_use_bou: 0,
	};

	let (supply_fuel, supply_ammo, supply_plane) = match mode {
		KcApiChargeKind::Fuel => (true, false, false),
		KcApiChargeKind::Ammo => (false, true, false),
		KcApiChargeKind::Plane => (false, false, true),
		KcApiChargeKind::Collective => (true, true, true),
	};

	// 0: fuel, 1: ammo, 2: steel, 3: bauxite
	let mut material_consumes = [0; 4];

	for ship in ships {
		let mst = codex.find::<ApiMstShip>(&ship.mst_id)?;
		let mut am = ship.into_active_model();
		let ratio = if ship.level > 99 {
			0.85
		} else {
			1.0
		};

		if supply_fuel {
			let max_fuel = mst.api_fuel_max.ok_or_else(|| {
				GameplayError::BadManifest(format!(
					"invalid fuel max for ship ID {}: {:?}",
					ship.mst_id, mst
				))
			})?;

			let consumption = max_fuel - ship.fuel;
			let consumption = ((consumption as f64 * ratio).floor() as i64).max(1);
			if consumption > 0 {
				material_consumes[0] += consumption;
				am.fuel = ActiveValue::Set(max_fuel);
			}
		}
		if supply_ammo {
			let max_ammo = mst.api_bull_max.ok_or_else(|| {
				GameplayError::BadManifest(format!(
					"invalid ammo max for ship ID {}: {:?}",
					ship.mst_id, mst
				))
			})?;

			let consumption = max_ammo - ship.ammo;
			let consumption = ((consumption as f64 * ratio).floor() as i64).max(1);
			if consumption > 0 {
				material_consumes[1] += consumption;
				am.ammo = ActiveValue::Set(max_ammo);
			}
		}
		if supply_aircrafts || supply_plane {
			let max_eq = mst.api_maxeq.unwrap_or([0; 5]);
			let mut plane_lost = 0;
			[(max_eq[0], ship.onslot_1, &mut am.onslot_1)].into_iter().for_each(
				|(max, current, am_current)| {
					let lost = max - current;
					if lost > 0 {
						plane_lost += lost;
						*am_current = ActiveValue::Set(max);
					}
				},
			);

			material_consumes[3] += plane_lost * 5;
		}

		let m = am.update(c).await?;

		resp.api_ship.push(KcApiChargeShip {
			api_id: m.id,
			api_fuel: m.fuel,
			api_bull: m.ammo,
			api_onslot: [m.onslot_1, m.onslot_2, m.onslot_3, m.onslot_4, m.onslot_5],
		});
	}

	let new_materials = deduct_material_impl(
		c,
		profile_id,
		&[
			(MaterialCategory::Fuel, material_consumes[0]),
			(MaterialCategory::Ammo, material_consumes[1]),
			(MaterialCategory::Steel, material_consumes[2]),
			(MaterialCategory::Bauxite, material_consumes[3]),
		],
	)
	.await?;

	resp.api_material[0] = new_materials.fuel;
	resp.api_material[1] = new_materials.ammo;
	resp.api_material[2] = new_materials.steel;
	resp.api_material[3] = new_materials.bauxite;

	resp.api_use_bou = material_consumes[3];

	Ok(resp)
}
