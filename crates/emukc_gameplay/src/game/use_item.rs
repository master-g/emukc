use async_trait::async_trait;
use rand::{rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};

use emukc_db::{
	entity::profile::item::use_item::{self, ActiveModel},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, TransactionTrait, TryIntoModel},
};
use emukc_model::{
	prelude::*,
	profile::{material::Material, user_item::UserItem},
};

use crate::game::material::{add_material_impl, get_mat_impl};
use crate::game::slot_item::add_slot_item_impl;
use crate::gameplay::HasContext;
use crate::{err::GameplayError, game::basic::inc_parallel_quest_max_impl};

use super::fleet::get_fleet_ships_impl;

/// A trait for use item related gameplay.
#[async_trait]
pub trait UseItemOps {
	/// Add use item to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The use item manifest ID.
	/// - `amount`: The amount of the use item.
	async fn add_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Find use item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The use item manifest ID.
	async fn find_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Get all use items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_use_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError>;

	/// Deduct use item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The use item manifest ID.
	/// - `amount`: The amount of the use item.
	async fn deduct_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Consume use item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The use item manifest ID.
	/// - `use_type`: The use type.
	/// - `force`: The force flag.
	async fn consume_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		use_type: i64,
		forced: bool,
	) -> Result<KcApiUseItemResp, GameplayError>;

	/// Consume Irako and/or Mamiya use item.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `deck_id`: The deck ID.
	/// - `use_type`: The use type.
	async fn consume_cond_use_item(
		&self,
		profile_id: i64,
		deck_id: i64,
		use_type: i64,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> UseItemOps for T {
	async fn add_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let am = add_use_item_impl(&tx, profile_id, mst_id, amount).await?;

		tx.commit().await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn find_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let am = find_use_item_impl(db, profile_id, mst_id).await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn get_use_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError> {
		let db = self.db();
		let items = get_use_items_impl(db, profile_id).await?;

		let items: Vec<UserItem> = items.into_iter().map(std::convert::Into::into).collect();
		let items: Vec<KcApiUserItem> = items.into_iter().map(std::convert::Into::into).collect();

		Ok(items)
	}

	async fn deduct_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let am = deduct_use_item_impl(&tx, profile_id, mst_id, amount).await?;

		tx.commit().await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn consume_use_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		exchange_type: i64,
		forced: bool,
	) -> Result<KcApiUseItemResp, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let resp =
			consume_use_item_impl(&tx, codex, profile_id, mst_id, exchange_type, forced).await?;

		tx.commit().await?;

		Ok(resp)
	}

	async fn consume_cond_use_item(
		&self,
		profile_id: i64,
		deck_id: i64,
		use_type: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		consume_cond_use_item_impl(&tx, profile_id, deck_id, use_type).await?;

		tx.commit().await?;

		Ok(())
	}
}

/// Add use item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `count`: The count of the item.
#[allow(unused)]
pub(crate) async fn add_use_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	amount: i64,
) -> Result<use_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;

	let am = match record {
		Some(rec) => ActiveModel {
			id: ActiveValue::Unchanged(rec.id),
			count: ActiveValue::Set(rec.count + amount),
			..rec.into()
		},
		None => use_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			mst_id: ActiveValue::Set(mst_id),
			count: ActiveValue::Set(amount),
		},
	};

	let model = am.save(c).await?;

	Ok(model.try_into_model()?)
}

pub(crate) async fn deduct_use_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	amount: i64,
) -> Result<use_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"use item: {} for profile: {}",
				mst_id, profile_id
			))
		})?;

	let new_amount = record.count - amount;
	if new_amount < 0 {
		return Err(GameplayError::Insufficient(format!(
			"use item: {} for profile: {}",
			mst_id, profile_id
		)));
	}

	let mut am = record.into_active_model();
	am.count = ActiveValue::Set(new_amount);

	let m = am.update(c).await?;

	Ok(m)
}

/// Find use item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
#[allow(unused)]
pub(crate) async fn find_use_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
) -> Result<use_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;
	let m = m.unwrap_or(use_item::Model {
		id: 0,
		profile_id,
		mst_id,
		count: 0,
	});
	Ok(m)
}

pub(crate) async fn get_use_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<use_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let items =
		use_item::Entity::find().filter(use_item::Column::ProfileId.eq(profile_id)).all(c).await?;
	Ok(items)
}

pub(crate) async fn consume_use_item_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	mst_id: i64,
	exchange_type: i64,
	forced: bool,
) -> Result<KcApiUseItemResp, GameplayError>
where
	C: ConnectionTrait,
{
	let use_item_type = KcUseItemType::n(mst_id).ok_or_else(|| {
		GameplayError::WrongType(format!("use item type: {} for profile: {}", mst_id, profile_id))
	})?;

	let use_item_model = find_use_item_impl(c, profile_id, mst_id).await?;

	let mut get_materials = Material::default();
	let mut get_items: Vec<KcApiUseItemGetItemElement> = vec![];

	let consume_amount = match use_item_type {
		KcUseItemType::FCoinBox200 | KcUseItemType::FCoinBox400 | KcUseItemType::FCoinBox700 => {
			// furniture coin box
			let single_amount = if use_item_type == KcUseItemType::FCoinBox200 {
				200
			} else if use_item_type == KcUseItemType::FCoinBox400 {
				400
			} else {
				700
			};

			let consume_amount = if exchange_type == 41 {
				// all
				use_item_model.count
			} else if exchange_type == 42 {
				// half
				use_item_model.count / 2
			} else if exchange_type == 43 {
				// 10
				use_item_model.count.min(10)
			} else {
				return Err(GameplayError::WrongType(format!(
					"exchange type: {} for use item: {}",
					exchange_type, mst_id
				)));
			};

			let api_getcount = single_amount * consume_amount;

			get_items.push(KcApiUseItemGetItemElement {
				api_usemst: 5,
				api_mst_id: KcUseItemType::FCoin as i64,
				api_getcount,
				api_slotitem: None,
			});

			consume_amount
		}
		KcUseItemType::Chocolate => {
			// chocolate
			get_materials.fuel = 700;
			get_materials.ammo = 700;
			get_materials.steel = 700;
			get_materials.bauxite = 1500;

			1
		}
		KcUseItemType::Medal => {
			// medal
			match exchange_type {
				0 => {
					// resource
					get_materials.fuel = 300;
					get_materials.ammo = 300;
					get_materials.steel = 300;
					get_materials.bauxite = 300;
					get_materials.bucket = 2;
					1
				}
				1 => {
					// 4 medal for 1 blueprint
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 6,
						api_mst_id: KcUseItemType::Blueprint as i64,
						api_getcount: 1,
						api_slotitem: None,
					});
					4
				}
				2 => {
					// screws
					get_materials.screw = 4;
					1
				}
				_ => {
					return Err(GameplayError::WrongType(format!(
						"exchange type: {} for use item: {}",
						exchange_type, mst_id
					)));
				}
			}
		}
		KcUseItemType::Presents => {
			// presents
			match exchange_type {
				11 => {
					// resource
					get_materials.fuel = 550;
					get_materials.ammo = 550;
				}
				12 => {
					// material
					get_materials.devmat = 3;
					get_materials.screw = 1;
				}
				13 => {
					// irako
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 5,
						api_mst_id: KcUseItemType::Irako as i64,
						api_getcount: 1,
						api_slotitem: None,
					});
				}
				_ => {
					return Err(GameplayError::WrongType(format!(
						"exchange type: {} for use item: {}",
						exchange_type, mst_id
					)));
				}
			};

			1
		}
		KcUseItemType::FirstClassMedal => {
			// first class medal
			get_materials.fuel = 10000;
			get_materials.devmat = 10;
			get_materials.screw = 10;

			get_items.push(KcApiUseItemGetItemElement {
				api_usemst: 5,
				api_mst_id: KcUseItemType::FCoinBox700 as i64,
				api_getcount: 10,
				api_slotitem: None,
			});

			1
		}
		KcUseItemType::Hishimochi => {
			// hishimochi
			match exchange_type {
				21 => {
					// resources
					get_materials.fuel = 600;
					get_materials.bauxite = 200;
				}
				22 => {
					// material
					get_materials.devmat = 1;
					get_materials.bucket = 2;
				}
				23 => {
					// irako
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 5,
						api_mst_id: KcUseItemType::Irako as i64,
						api_getcount: 1,
						api_slotitem: None,
					});
				}
				_ => {
					return Err(GameplayError::WrongType(format!(
						"exchange type: {} for use item: {}",
						exchange_type, mst_id
					)));
				}
			};

			1
		}
		KcUseItemType::HQPersonnel => {
			inc_parallel_quest_max_impl(c, codex, profile_id).await?;

			1
		}
		KcUseItemType::Saury => {
			match exchange_type {
				31 => {
					// sasimi
					get_materials.ammo = 300;
					get_materials.steel = 150;

					3
				}
				32 => {
					// shioyaki
					get_materials.devmat = 3;
					get_materials.screw = 1;

					5
				}
				33 => {
					// kabayaki
					// 秋刀魚の缶詰
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 2,
						api_mst_id: 150,
						api_getcount: 1,
						api_slotitem: Some(KcApiSlotItem {
							api_id: 0,
							api_slotitem_id: 150,
							api_locked: 0,
							api_level: 0,
							api_alv: None,
						}),
					});
					get_materials.bucket = 3;

					7
				}
				34 => {
					// grilled
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 2,
						api_mst_id: 388,
						api_getcount: 1,
						api_slotitem: Some(KcApiSlotItem {
							api_id: 0,
							api_slotitem_id: 388,
							api_locked: 0,
							api_level: 0,
							api_alv: None,
						}),
					});

					48
				}
				_ => {
					return Err(GameplayError::WrongType(format!(
						"exchange type: {} for use item: {}",
						exchange_type, mst_id
					)));
				}
			}
		}
		KcUseItemType::TeruteruBouzu => {
			match exchange_type {
				111 => {
					// furniture
					10
				}
				112 => {
					// slot item
					11
				}
				113 => {
					// blue ribbon
					get_items.push(KcApiUseItemGetItemElement {
						api_usemst: 5,
						api_mst_id: KcUseItemType::BlueRibbon as i64,
						api_getcount: 1,
						api_slotitem: None,
					});
					12
				}
				114 => {
					get_materials.bucket = 1;
					get_materials.screw = 1;

					1
				}
				_ => {
					return Err(GameplayError::WrongType(format!(
						"exchange type: {} for use item: {}",
						exchange_type, mst_id
					)));
				}
			}
		}
		_ => {
			error!("unhandled use item type: {}", mst_id);
			return Err(GameplayError::WrongType(format!(
				"do not know how to use item type: {} for profile: {}",
				mst_id, profile_id
			)));
		}
	};

	let mut api_caution_flag = 0;

	let owned_material_model = get_mat_impl(c, profile_id).await?;
	for after in [
		owned_material_model.fuel + get_materials.fuel,
		owned_material_model.ammo + get_materials.ammo,
		owned_material_model.steel + get_materials.steel,
		owned_material_model.bauxite + get_materials.bauxite,
	] {
		if after > codex.game_cfg.material.primary_resource_hard_cap {
			api_caution_flag = 1;
			break;
		}
	}

	for after in [
		owned_material_model.bucket + get_materials.bucket,
		owned_material_model.torch + get_materials.torch,
		owned_material_model.devmat + get_materials.devmat,
		owned_material_model.screw + get_materials.screw,
	] {
		if after > codex.game_cfg.material.special_resource_cap {
			api_caution_flag = 1;
			break;
		}
	}

	let mut api_flag = if get_items.is_empty() {
		0
	} else {
		1
	};

	let resp = if forced || api_caution_flag == 0 {
		// add items
		for item in get_items.iter_mut() {
			if let Some(slot_item) = &mut item.api_slotitem {
				let m = add_slot_item_impl(
					c,
					codex,
					profile_id,
					slot_item.api_slotitem_id,
					slot_item.api_level,
					0,
				)
				.await?;
				slot_item.api_id = m.id;
			} else {
				add_use_item_impl(c, profile_id, item.api_mst_id, item.api_getcount).await?;
			}
		}

		// deduct use item
		deduct_use_item_impl(c, profile_id, mst_id, consume_amount).await?;

		// add materials
		let mats: Vec<(MaterialCategory, i64)> = [
			(MaterialCategory::Fuel, get_materials.fuel),
			(MaterialCategory::Ammo, get_materials.ammo),
			(MaterialCategory::Steel, get_materials.steel),
			(MaterialCategory::Bauxite, get_materials.bauxite),
			(MaterialCategory::Torch, get_materials.torch),
			(MaterialCategory::Bucket, get_materials.bucket),
			(MaterialCategory::DevMat, get_materials.devmat),
		]
		.into_iter()
		.filter(|(_, amount)| *amount > 0)
		.collect();

		if !mats.is_empty() {
			api_flag = if api_flag == 0 {
				2
			} else {
				3
			};
			add_material_impl(c, codex, profile_id, &mats).await?;
		}

		KcApiUseItemResp {
			api_caution_flag: 0,
			api_material: get_materials.into_array(),
			api_flag,
			api_getitem: if get_items.is_empty() {
				None
			} else {
				Some(get_items)
			},
		}
	} else {
		KcApiUseItemResp {
			api_caution_flag: 1,
			api_flag: 0,
			api_getitem: None,
			api_material: [0; 8],
		}
	};

	Ok(resp)
}

pub(crate) async fn consume_cond_use_item_impl<C>(
	c: &C,
	profile_id: i64,
	deck_id: i64,
	use_type: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let mut ships = get_fleet_ships_impl(c, profile_id, deck_id).await?;

	deduct_use_item_impl(c, profile_id, KcUseItemType::Mamiya as i64, 1).await?;
	deduct_use_item_impl(c, profile_id, KcUseItemType::Irako as i64, 1).await?;

	match use_type {
		1 => {
			// mamiya
			ships.iter_mut().for_each(|ship| {
				ship.condition = if ship.condition < 40 {
					40
				} else if ship.condition < 50 {
					50
				} else {
					ship.condition
				}
			});
		}
		2 => {
			// irako
			let mut rng = SmallRng::from_os_rng();
			let lucky_count = rng.random_range(0..=3);
			let mut luck_ship_id = vec![ships[0].id];
			for _ in 0..lucky_count {
				let id = ships.iter().skip(1).choose(&mut rng).unwrap().id;
				if !luck_ship_id.contains(&id) {
					luck_ship_id.push(id);
				}
			}
			for id in luck_ship_id {
				let ship = ships.iter_mut().find(|s| s.id == id).unwrap();
				ship.condition = if ship.condition <= 75 {
					ship.condition + 25
				} else {
					100
				};
			}
		}
		3 => {
			// mamiya + irako
			let mut rng = SmallRng::from_os_rng();
			ships.iter_mut().enumerate().for_each(|(i, ship)| {
				if i == 0 {
					if ship.condition < 40 {
						ship.condition = 70;
					} else {
						ship.condition += 30;
					}
				} else {
					if ship.condition < 40 {
						ship.condition = 40;
					}
					ship.condition += rng.random_range(20..=31);
				}
				ship.condition = ship.condition.min(100);
			});
		}
		_ => {
			return Err(GameplayError::WrongType(format!(
				"use type: {} for profile: {}",
				use_type, profile_id
			)));
		}
	}

	for ship in ships {
		let cond = ship.condition;
		let mut am = ship.into_active_model();
		am.condition = ActiveValue::Set(cond);

		am.update(c).await?;
	}

	Ok(())
}

pub(super) async fn init<C>(_c: &C, _profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	use_item::Entity::delete_many()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
