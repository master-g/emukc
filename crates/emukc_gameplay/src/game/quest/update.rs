use async_trait::async_trait;
use emukc_db::{
	entity::profile::{expedition, quest},
	sea_orm::{entity::prelude::*, TransactionTrait},
};

use crate::{err::GameplayError, gameplay::HasContext};

pub(crate) async fn update_quests_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	todo!()
}
