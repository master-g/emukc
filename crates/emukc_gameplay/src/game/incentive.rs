//! Incentive related gameplay operations.

use emukc_db::{
	entity::profile::incentive::{self, IncentiveMode, IncentiveType},
	sea_orm::{QueryOrder, TransactionTrait, entity::*},
};
use emukc_model::kc2::{KcApiIncentiveItem, MaterialCategory};
use prelude::{ConnectionTrait, QueryFilter, async_trait::async_trait};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	furniture::add_furniture_impl, material::add_material_impl, ship::add_ship_impl,
	slot_item::add_slot_item_impl, use_item::add_use_item_impl,
};

/// A trait for incentive related gameplay.
#[async_trait]
pub trait IncentiveOps {
	/// Add incentives to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `items`: The items to add.
	async fn add_incentive(
		&self,
		profile_id: i64,
		items: &[KcApiIncentiveItem],
	) -> Result<(), GameplayError>;

	/// Confirm incentives for a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn confirm_incentives(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiIncentiveItem>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> IncentiveOps for T {
	async fn add_incentive(
		&self,
		profile_id: i64,
		items: &[KcApiIncentiveItem],
	) -> Result<(), GameplayError> {
		let db = self.db();

		let tx = db.begin().await?;

		let ams: Vec<incentive::ActiveModel> = items
			.iter()
			.filter_map(|i| {
				let mode = IncentiveMode::n(i.api_mode).or_else(|| {
					error!("Invalid incentive mode: {}", i.api_mode);
					None
				})?;
				let typ = IncentiveType::n(i.api_type).or_else(|| {
					error!("Invalid incentive type: {}", i.api_type);
					None
				})?;
				Some(incentive::ActiveModel {
					id: ActiveValue::NotSet,
					profile_id: ActiveValue::Set(profile_id),
					mode: ActiveValue::Set(mode),
					typ: ActiveValue::Set(typ),
					mst_id: ActiveValue::Set(i.api_mst_id),
					amount: ActiveValue::Set(i.amount),
					stars: ActiveValue::Set(i.api_slotitem_level),
					alv: ActiveValue::Set((i.alv > 0).then_some(i.alv)),
				})
			})
			.collect();

		if ams.is_empty() {
			error!("No valid incentives to add");
			return Ok(());
		}

		incentive::Entity::insert_many(ams).exec(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn confirm_incentives(
		&self,
		profile_id: i64,
	) -> Result<Vec<KcApiIncentiveItem>, GameplayError> {
		let db = self.db();
		let codex = self.codex();

		let tx = db.begin().await?;

		let items = incentive::Entity::find()
			.filter(incentive::Column::ProfileId.eq(profile_id))
			.order_by_asc(incentive::Column::Id)
			.all(&tx)
			.await?;

		let api_items: Vec<KcApiIncentiveItem> = items
			.iter()
			.map(|i| KcApiIncentiveItem {
				api_mode: i.mode as i64,
				api_type: i.typ as i64,
				api_mst_id: i.mst_id,
				api_slotitem_level: i.stars,
				api_getmes: if i.typ == IncentiveType::Ship {
					codex
						.manifest
						.api_mst_ship
						.iter()
						.find(|s| s.api_id == i.mst_id)
						.unwrap()
						.api_getmes
						.clone()
				} else {
					None
				},
				amount: i.amount,
				alv: i.alv.unwrap_or_default(),
			})
			.collect();

		// apply incentives
		for item in items {
			match item.typ {
				IncentiveType::Ship => {
					add_ship_impl(&tx, codex, profile_id, item.mst_id).await?;
				}
				IncentiveType::SlotItem => {
					add_slot_item_impl(
						&tx,
						codex,
						profile_id,
						item.mst_id,
						item.stars.unwrap_or_default(),
						item.alv.unwrap_or_default(),
					)
					.await?;
				}
				IncentiveType::UseItem => {
					add_use_item_impl(&tx, profile_id, item.mst_id, item.amount).await?;
				}
				IncentiveType::Resource => {
					let category = MaterialCategory::n(item.mst_id).ok_or_else(|| {
						GameplayError::WrongType(format!(
							"invalid material category: {}",
							item.mst_id
						))
					})?;
					add_material_impl(&tx, codex, profile_id, &[(category, item.amount)]).await?;
				}
				IncentiveType::Furniture => {
					add_furniture_impl(&tx, profile_id, item.mst_id).await?;
				}
			}
		}

		// remove incentives
		incentive::Entity::delete_many()
			.filter(incentive::Column::ProfileId.eq(profile_id))
			.exec(&tx)
			.await?;

		tx.commit().await?;

		Ok(api_items)
	}
}

pub(super) async fn init<C>(_c: &C, _profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// this function is empty for now
	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	incentive::Entity::delete_many()
		.filter(incentive::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
