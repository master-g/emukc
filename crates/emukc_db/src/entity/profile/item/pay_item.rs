//! Use Item Entity

use emukc_model::profile::user_item::UserItem;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "pay_item")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// Item count
	pub count: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<UserItem> for ActiveModel {
	fn from(t: UserItem) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(t.id),
			mst_id: ActiveValue::Set(t.mst_id),
			count: ActiveValue::Set(t.count),
		}
	}
}

impl From<Model> for UserItem {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			mst_id: value.mst_id,
			count: value.count,
		}
	}
}
