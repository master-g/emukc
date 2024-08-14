//! Use Item Entity

use emukc_model::profile::user_item::UserItem;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "use_item")]
pub struct Model {
	/// Manifest ID
	#[sea_orm(primary_key)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Item count
	pub count: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(belongs_to = "super::Entity", from = "Column::ProfileId", to = "super::Column::Id")]
	Profile,
}

impl Related<super::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<UserItem> for ActiveModel {
	fn from(t: UserItem) -> Self {
		Self {
			id: ActiveValue::Set(t.mst_id),
			profile_id: ActiveValue::Set(t.id),
			count: ActiveValue::Set(t.count),
		}
	}
}
