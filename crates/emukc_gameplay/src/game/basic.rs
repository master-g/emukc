use async_trait::async_trait;
use emukc_crypto::SimpleHash;
use emukc_db::{
	entity::profile::{self, kdock},
	sea_orm::{entity::prelude::*, TransactionTrait},
};
use emukc_model::kc2::KcApiUserBasic;

use crate::{err::GameplayError, prelude::HasContext};

use super::{furniture::get_furniture_config_impl, kdock::get_kdocks_impl};

/// A trait for furniture related gameplay.
#[async_trait]
pub trait BasicOps {
	/// Get user basics.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_user_basic(&self, profile_id: i64) -> Result<KcApiUserBasic, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> BasicOps for T {
	async fn get_user_basic(&self, profile_id: i64) -> Result<KcApiUserBasic, GameplayError> {
		let db = self.db();

		let tx = db.begin().await?;

		let (_, basic) = get_user_basic_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(basic)
	}
}

/// Get user basics.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub async fn get_user_basic_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(profile::Model, KcApiUserBasic), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(record) =
		profile::Entity::find().filter(profile::Column::Id.eq(profile_id)).one(c).await?
	else {
		return Err(GameplayError::ProfileNotFound(profile_id));
	};

	// furniture
	let (_, furniture_cfg) = get_furniture_config_impl(c, profile_id).await?;
	// construction docks
	let kdocks = get_kdocks_impl(c, profile_id).await?;
	let api_count_kdock =
		kdocks.iter().filter(|x| x.status != kdock::Status::Locked).count() as i64;

	let basic = KcApiUserBasic {
		api_member_id: record.id,
		api_nickname: record.name.clone(),
		api_nickname_id: record.name.simple_hash(),
		api_active_flag: 1,
		api_starttime: record.last_played.timestamp(),
		api_level: record.hq_level,
		api_rank: record.hq_rank,
		api_experience: record.experience,
		api_fleetname: None,
		api_comment: record.comment.clone(),
		api_comment_id: record.comment.simple_hash(),
		api_max_chara: record.max_ship_capacity,
		api_max_slotitem: record.max_equipment_capacity,
		api_max_kagu: 0,
		api_playtime: 0,
		api_tutorial: record.tutorial_progress,
		api_furniture: furniture_cfg.api_values(),
		api_count_deck: 0, // needs to be filled in another api
		api_count_kdock,
		api_count_ndock: 0, // needs to be filled in another api
		api_fcoin: 0,       // needs to be filled in another api
		api_st_win: record.sortie_wins,
		api_st_lose: record.sortie_loses,
		api_ms_count: record.expeditions,
		api_ms_success: record.expeditions_success,
		api_pt_win: record.practice_battle_wins,
		api_pt_lose: record.practice_battles - record.practice_battle_wins,
		api_pt_challenged: record.practice_challenges,
		api_pt_challenged_win: record.practice_challenge_wins,
		api_firstflag: if record.intro_completed {
			1
		} else {
			0
		},
		api_tutorial_progress: record.tutorial_progress,
		api_pvp: [0, 0],
		api_medals: record.medals,
		api_large_dock: if record.large_dock_unlocked {
			1
		} else {
			0
		},
		api_max_quests: record.max_quests,
		api_extra_supply: [
			if record.extra_supply_expedition {
				1
			} else {
				0
			},
			if record.extra_supply_sortie {
				1
			} else {
				0
			},
		],
		api_war_result: record.war_result,
	};

	Ok((record, basic))
}
