use crate::err::GameplayError;
use crate::gameplay::HasContext;
use async_trait::async_trait;
use emukc_db::{
	entity::profile::item::pay_item::{self, ActiveModel},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait, TryIntoModel},
};
use emukc_model::{
	prelude::*,
	profile::{material::Material, user_item::UserItem},
};
use emukc_time::chrono::Utc;

use super::material::{add_material_impl, get_mat_impl};

/// A trait for pay item related gameplay.
#[async_trait]
pub trait PayItemOps {
	/// Add pay item to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The pay item manifest ID.
	/// - `amount`: The amount of the pay item.
	async fn add_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Find pay item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The pay item manifest ID.
	async fn find_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError>;

	/// Get all pay items from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_pay_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError>;

	/// Consume pay item from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The pay item manifest ID.
	/// - `forced`: Whether to force consume the item.
	async fn consume_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		forced: bool,
	) -> Result<bool, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PayItemOps for T {
	async fn add_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		amount: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let am = add_pay_item_impl(&tx, profile_id, mst_id, amount).await?;

		tx.commit().await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn find_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
	) -> Result<KcApiUserItem, GameplayError> {
		let db = self.db();
		let am = find_pay_item_impl(db, profile_id, mst_id).await?;

		Ok(KcApiUserItem {
			api_id: mst_id,
			api_count: am.count,
		})
	}

	async fn get_pay_items(&self, profile_id: i64) -> Result<Vec<KcApiUserItem>, GameplayError> {
		let db = self.db();
		let items = get_pay_items_impl(db, profile_id).await?;

		let items: Vec<UserItem> = items.into_iter().map(std::convert::Into::into).collect();
		let items: Vec<KcApiUserItem> = items.into_iter().map(std::convert::Into::into).collect();

		Ok(items)
	}

	async fn consume_pay_item(
		&self,
		profile_id: i64,
		mst_id: i64,
		forced: bool,
	) -> Result<bool, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let cauction = consume_pay_item_impl(&tx, codex, profile_id, mst_id, forced).await?;

		Ok(true)
	}
}

/// Add pay item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `count`: The count of the item.
#[allow(unused)]
pub(crate) async fn add_pay_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	amount: i64,
) -> Result<pay_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = pay_item::Entity::find()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.filter(pay_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;

	let am = match record {
		Some(rec) => ActiveModel {
			id: ActiveValue::Unchanged(rec.id),
			count: ActiveValue::Set(rec.count + amount),
			..rec.into()
		},
		None => pay_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			mst_id: ActiveValue::Set(mst_id),
			count: ActiveValue::Set(amount),
		},
	};

	let model = am.save(c).await?;

	Ok(model.try_into_model()?)
}

/// Find pay item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
#[allow(unused)]
pub(crate) async fn find_pay_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
) -> Result<pay_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let m = pay_item::Entity::find()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.filter(pay_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;
	let m = m.unwrap_or(pay_item::Model {
		id: 0,
		profile_id,
		mst_id,
		count: 0,
	});
	Ok(m)
}

pub(crate) async fn get_pay_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<pay_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let items =
		pay_item::Entity::find().filter(pay_item::Column::ProfileId.eq(profile_id)).all(c).await?;
	Ok(items)
}

pub(crate) async fn consume_pay_item_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	mst_id: i64,
	forced: bool,
) -> Result<bool, GameplayError>
where
	C: ConnectionTrait,
{
	let item = find_pay_item_impl(c, profile_id, mst_id).await?;
	if item.count <= 0 {
		return Ok(false);
	}

	let pay_item_type = KcPayItemType::n(mst_id)
		.ok_or_else(|| GameplayError::WrongType(format!("Invalid pay item type: {}", mst_id)))?;

	let mst = codex.find::<ApiMstPayitem>(&mst_id)?;

	let get_materials = Material {
		id: 0,
		last_update_primary: Utc::now(),
		last_update_bauxite: Utc::now(),
		fuel: mst.api_item[0],
		ammo: mst.api_item[1],
		steel: mst.api_item[2],
		bauxite: mst.api_item[3],
		torch: mst.api_item[4],
		bucket: mst.api_item[5],
		devmat: mst.api_item[6],
		screw: mst.api_item[7],
	};

	let owned_material_model = get_mat_impl(c, profile_id).await?;
	let mut caution = false;
	for after in [
		owned_material_model.fuel + get_materials.fuel,
		owned_material_model.ammo + get_materials.ammo,
		owned_material_model.steel + get_materials.steel,
		owned_material_model.bauxite + get_materials.bauxite,
	] {
		if after > codex.game_cfg.material.primary_resource_hard_cap {
			caution = true;
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
			caution = true;
			break;
		}
	}

	if !forced && caution {
		return Ok(true);
	}

	// add materials
	add_material_impl(
		c,
		codex,
		profile_id,
		&[
			(MaterialCategory::Fuel, get_materials.fuel),
			(MaterialCategory::Ammo, get_materials.ammo),
			(MaterialCategory::Steel, get_materials.steel),
			(MaterialCategory::Bauxite, get_materials.bauxite),
			(MaterialCategory::Torch, get_materials.torch),
			(MaterialCategory::Bucket, get_materials.bucket),
			(MaterialCategory::DevMat, get_materials.devmat),
			(MaterialCategory::Screw, get_materials.screw),
		],
	)
	.await?;

	match pay_item_type {
		KcPayItemType::FuelPack => todo!(),
		KcPayItemType::AmmoPack => todo!(),
		KcPayItemType::SteelPack => todo!(),
		KcPayItemType::BauxitePack => todo!(),
		KcPayItemType::DevMaterialPack => todo!(),
		KcPayItemType::TorchPack => todo!(),
		KcPayItemType::BucketPack => todo!(),
		KcPayItemType::FactorySet => todo!(),
		KcPayItemType::SortieSet => todo!(),
		KcPayItemType::DockExpansionSet => todo!(),
		KcPayItemType::RepairTeam => todo!(),
		KcPayItemType::RepairSpecialSet => todo!(),
		KcPayItemType::EightyEightResourceSet => todo!(),
		KcPayItemType::RepairGoddess => todo!(),
		KcPayItemType::FurnitureCraftman => todo!(),
		KcPayItemType::PortExpansion => todo!(),
		KcPayItemType::TankerRequisition => todo!(),
		KcPayItemType::Mamiya => todo!(),
		KcPayItemType::AluminumMassProduction => todo!(),
		KcPayItemType::Ring => todo!(),
		KcPayItemType::Cookie => todo!(),
		KcPayItemType::Screw10 => todo!(),
		KcPayItemType::Irako5 => todo!(),
		KcPayItemType::BattleRation => todo!(),
		KcPayItemType::Resupplier => todo!(),
		KcPayItemType::ReinforceExpansion => todo!(),
		KcPayItemType::ConstCorps => todo!(),
		KcPayItemType::EmergencyRepairMaterialSet => todo!(),
		KcPayItemType::SubmarineSupplyMaterialPack => todo!(),
	};

	Ok(false)
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
	pay_item::Entity::delete_many()
		.filter(pay_item::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
