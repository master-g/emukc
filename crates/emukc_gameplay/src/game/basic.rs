use async_trait::async_trait;
use emukc_crypto::SimpleHash;
use emukc_db::{
	entity::profile::{self, kdock, ndock},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::kc2::{KcApiUserBasic, KcUseItemType};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	fleet::get_fleets_impl, furniture::get_furniture_config_impl, kdock::get_kdocks_impl,
	ndock::get_ndocks_impl, use_item::find_use_item_impl,
};

/// A trait for furniture related gameplay.
#[async_trait]
pub trait BasicOps {
	/// Get user basics.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_user_basic(
		&self,
		profile_id: i64,
	) -> Result<(profile::Model, KcApiUserBasic), GameplayError>;

	/// Update user nickname.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `nickname`: The new nickname.
	async fn update_user_nickname(
		&self,
		profile_id: i64,
		nickname: &str,
	) -> Result<(), GameplayError>;

	/// Update user comment.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `comment`: The new comment.
	async fn update_user_comment(
		&self,
		profile_id: i64,
		comment: &str,
	) -> Result<(), GameplayError>;

	/// Update user firstflag.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `firstflag`: The new firstflag.
	async fn update_user_first_flag(
		&self,
		profile_id: i64,
		firstflag: i64,
	) -> Result<(), GameplayError>;

	/// Update user tutorial progress.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `progress`: The new tutorial progress.
	async fn update_tutorial_progress(
		&self,
		profile_id: i64,
		progress: i64,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> BasicOps for T {
	async fn get_user_basic(
		&self,
		profile_id: i64,
	) -> Result<(profile::Model, KcApiUserBasic), GameplayError> {
		let db = self.db();

		let (model, basic) = get_user_basic_impl(db, profile_id).await?;

		Ok((model, basic))
	}

	async fn update_user_nickname(
		&self,
		profile_id: i64,
		nickname: &str,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_user_nickname_impl(&tx, profile_id, nickname).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_user_comment(
		&self,
		profile_id: i64,
		comment: &str,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_user_comment_impl(&tx, profile_id, comment).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_user_first_flag(
		&self,
		profile_id: i64,
		firstflag: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_user_first_flag_impl(&tx, profile_id, firstflag).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_tutorial_progress(
		&self,
		profile_id: i64,
		progress: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_tutorial_progress_impl(&tx, profile_id, progress).await?;

		tx.commit().await?;

		Ok(())
	}
}

/// Get user basics.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub(crate) async fn get_user_basic_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(profile::Model, KcApiUserBasic), GameplayError>
where
	C: ConnectionTrait,
{
	let record = find_profile(c, profile_id).await?;

	// furniture
	let (_, furniture_cfg) = get_furniture_config_impl(c, profile_id).await?;
	// fleets
	let fleets = get_fleets_impl(c, profile_id).await?;
	let api_count_deck = fleets.len() as i64;
	// construction docks
	let kdocks = get_kdocks_impl(c, profile_id).await?;
	let api_count_kdock =
		kdocks.iter().filter(|x| x.status != kdock::Status::Locked).count() as i64;
	// repair docks
	let ndocks = get_ndocks_impl(c, profile_id).await?;
	let api_count_ndock =
		ndocks.iter().filter(|x| x.status != ndock::Status::Locked).count() as i64;
	// fcoin
	let fcoin = find_use_item_impl(c, profile_id, KcUseItemType::FCoin as i64).await?;
	let api_fcoin = fcoin.count;

	let basic = KcApiUserBasic {
		api_member_id: record.id,
		api_nickname: record.name.clone(),
		api_nickname_id: record.name.hash_i64().to_string(),
		api_active_flag: 1,
		api_starttime: record.last_played.timestamp(),
		api_level: record.hq_level,
		api_rank: record.hq_rank,
		api_experience: record.experience,
		api_fleetname: None,
		api_comment: record.comment.clone(),
		api_comment_id: record.comment.hash_i64().to_string(),
		api_max_chara: record.max_ship_capacity,
		api_max_slotitem: record.max_equipment_capacity,
		api_max_kagu: 0,
		api_playtime: 0,
		api_tutorial: record.tutorial_progress,
		api_furniture: furniture_cfg.api_values(),
		api_count_deck,
		api_count_kdock,
		api_count_ndock,
		api_fcoin,
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

pub(super) async fn find_profile<C>(c: &C, profile_id: i64) -> Result<profile::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = profile::Entity::find()
		.filter(profile::Column::Id.eq(profile_id))
		.one(c)
		.await?
		.ok_or(GameplayError::ProfileNotFound(profile_id))?;

	Ok(record)
}

pub(crate) async fn update_user_nickname_impl<C>(
	c: &C,
	profile_id: i64,
	nickname: &str,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let profile = find_profile(c, profile_id).await?;

	let mut am: profile::ActiveModel = profile.into();

	am.id = ActiveValue::Unchanged(profile_id);
	am.name = ActiveValue::Set(nickname.to_string());

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn update_user_comment_impl<C>(
	c: &C,
	profile_id: i64,
	comment: &str,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let profile = find_profile(c, profile_id).await?;

	let mut am: profile::ActiveModel = profile.into();

	am.id = ActiveValue::Unchanged(profile_id);
	am.comment = ActiveValue::Set(comment.to_string());

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn update_user_first_flag_impl<C>(
	c: &C,
	profile_id: i64,
	firstflag: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let profile = find_profile(c, profile_id).await?;
	let mut am: profile::ActiveModel = profile.into();

	am.id = ActiveValue::Unchanged(profile_id);
	am.intro_completed = ActiveValue::Set(firstflag != 0);

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn update_tutorial_progress_impl<C>(
	c: &C,
	profile_id: i64,
	tutorial_progress: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let profile = find_profile(c, profile_id).await?;
	let mut am: profile::ActiveModel = profile.into();

	am.id = ActiveValue::Unchanged(profile_id);
	am.tutorial_progress = ActiveValue::Set(tutorial_progress);

	am.update(c).await?;

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
	let profile = find_profile(c, profile_id).await?;
	let mut am: profile::ActiveModel =
		profile::default_active_model(profile.account_id, &profile.name);
	am.id = ActiveValue::Unchanged(profile_id);

	am.update(c).await?;

	Ok(())
}
