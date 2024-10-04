//! Incentive related gameplay operations.

use emukc_db::{
	entity::profile::incentive::{self, IncentiveMode, IncentiveType},
	sea_orm::{entity::*, QueryOrder, TransactionTrait},
};
use emukc_model::kc2::KcApiIncentiveItem;
use prelude::{async_trait::async_trait, QueryFilter};

use crate::{err::GameplayError, prelude::HasContext};

use super::use_item::add_use_item;

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
				let Some(mode) = IncentiveMode::n(i.api_mode) else {
					error!("Invalid incentive mode: {}", i.api_mode);
					return None;
				};
				let Some(typ) = IncentiveType::n(i.api_type) else {
					error!("Invalid incentive type: {}", i.api_type);
					return None;
				};
				Some(incentive::ActiveModel {
					id: ActiveValue::NotSet,
					profile_id: ActiveValue::Set(profile_id),
					mode: ActiveValue::Set(mode),
					typ: ActiveValue::Set(typ),
					mst_id: ActiveValue::Set(i.api_mst_id),
					stars: ActiveValue::Set(i.api_slotitem_level),
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
			})
			.collect();

		for item in items {
			match item.typ {
				IncentiveType::Ship => todo!(),
				IncentiveType::SlotItem => {
					add_use_item(&tx, profile_id, item.mst_id, item.stars.unwrap_or_default())
						.await?;
				}
				IncentiveType::UseItem => {
					add_use_item(&tx, profile_id, item.mst_id, 1).await?;
				}
				IncentiveType::Resource => todo!(),
				IncentiveType::Furniture => todo!(),
			}
		}

		// TODO: apply and then remove incentives

		Ok(api_items)
	}
}
