use async_trait::async_trait;
use emukc_db::sea_orm::{entity::prelude::*, TransactionTrait};
use emukc_model::{kc2::MaterialCategory, profile::material::Material};

use crate::{
	err::GameplayError,
	game::{material::deduct_material_impl, slot_item::add_slot_item_impl},
	gameplay::HasContext,
};

/// A trait for factory related gameplay.
#[async_trait]
pub trait FactoryOps {
	/// Create slot items.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The slot item manifest ID.
	/// - `consumption`: The materials consumption.
	async fn create_slotitem(
		&self,
		profile_id: i64,
		mst_id: &[i64],
		consumption: Vec<(MaterialCategory, i64)>,
	) -> Result<(Vec<i64>, Material), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> FactoryOps for T {
	async fn create_slotitem(
		&self,
		profile_id: i64,
		mst_id: &[i64],
		consumption: Vec<(MaterialCategory, i64)>,
	) -> Result<(Vec<i64>, Material), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let mut slot_ids: Vec<i64> = Vec::new();

		// deduct material consumption
		let m = deduct_material_impl(&tx, profile_id, consumption).await?;
		let m: Material = m.into();

		// add items
		for id in mst_id.iter() {
			if *id > 0 {
				let m = add_slot_item_impl(&tx, codex, profile_id, *id, 0, 0).await?;
				slot_ids.push(m.id);
			} else {
				slot_ids.push(*id);
			}
		}

		tx.commit().await?;

		Ok((slot_ids, m))
	}
}
