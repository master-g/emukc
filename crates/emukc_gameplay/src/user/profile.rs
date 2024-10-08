//! Deal with account related stuff.

use emukc_db::{
	entity::profile,
	sea_orm::{entity::*, query::*},
};
use emukc_model::{
	profile::Profile,
	user::token::{Token, TokenType},
};
use emukc_time::chrono::Utc;
use prelude::async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::gameplay::HasContext;

use super::{
	auth::{issue_token, verify_access_token},
	UserError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartGameInfo {
	pub profile: Profile,
	pub session: Token,
}

/// A trait for account related gameplay.
#[async_trait]
pub trait ProfileOps {
	/// Create a new profile.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token of the account.
	/// * `profile_name` - The name of the new profile.
	async fn new_profile(
		&self,
		access_token: &str,
		profile_name: &str,
	) -> Result<StartGameInfo, UserError>;

	/// Start a game session.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token of the account.
	/// * `profile_id` - The profile ID to start the game with.
	async fn start_game(
		&self,
		access_token: &str,
		profile_id: i64,
	) -> Result<StartGameInfo, UserError>;

	/// Select a world for the profile.
	///
	/// # Arguments
	///
	/// * `profile_id` - The profile ID to select the world for.
	/// * `world_id` - The world ID to select.
	async fn select_world(&self, profile_id: i64, world_id: i64) -> Result<(), UserError>;

	/// Remove an profile and all its data.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token of the account.
	async fn delete_profile(&self, access_token: &str) -> Result<(), UserError>;

	/// Find a profile by its ID.
	///
	/// # Arguments
	///
	/// * `profile_id` - The profile ID to find.
	async fn find_profile(&self, profile_id: i64) -> Result<Profile, UserError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ProfileOps for T {
	async fn new_profile(
		&self,
		access_token: &str,
		profile_name: &str,
	) -> Result<StartGameInfo, UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		// verify access token
		let account_model = verify_access_token(&tx, access_token).await?;

		// find profile
		let profile_model = profile::Entity::find()
			.filter(profile::Column::AccountId.eq(account_model.uid))
			.filter(profile::Column::Name.eq(profile_name))
			.one(&tx)
			.await?;

		if profile_model.is_some() {
			return Err(UserError::ProfileExists);
		}

		let am = profile::ActiveModel {
			id: ActiveValue::NotSet,
			account_id: ActiveValue::Set(account_model.uid),
			world_id: ActiveValue::Set(0),
			name: ActiveValue::Set(profile_name.to_string()),
			last_played: ActiveValue::Set(Utc::now()),
			hq_level: ActiveValue::Set(1),
			hq_rank: ActiveValue::Set(0),
			experience: ActiveValue::Set(0),
			comment: ActiveValue::Set("".to_string()),
			max_ship_capacity: ActiveValue::Set(100),
			max_equipment_capacity: ActiveValue::Set(497),
			deck_num: ActiveValue::Set(1),
			kdock_num: ActiveValue::Set(2),
			ndock_num: ActiveValue::Set(2),
			sortie_wins: ActiveValue::Set(0),
			expeditions: ActiveValue::Set(0),
			expeditions_success: ActiveValue::Set(0),
			practice_battles: ActiveValue::Set(0),
			practice_battle_wins: ActiveValue::Set(0),
			practice_challenges: ActiveValue::Set(0),
			practice_challenge_wins: ActiveValue::Set(0),
			intro_completed: ActiveValue::Set(false),
			tutorial_progress: ActiveValue::Set(0),
			medals: ActiveValue::Set(0),
			large_dock_unlocked: ActiveValue::Set(false),
			max_quests: ActiveValue::Set(5),
			extra_supply_expedition: ActiveValue::Set(false),
			extra_supply_sortie: ActiveValue::Set(false),
			war_result: ActiveValue::Set(0),
		};

		let profile_model = am.insert(&tx).await?;

		// issue new tokens
		let session =
			issue_token(&tx, account_model.uid, profile_model.id, TokenType::Session).await?;

		tx.commit().await?;

		Ok(StartGameInfo {
			profile: profile_model.into(),
			session,
		})
	}

	async fn start_game(
		&self,
		access_token: &str,
		profile_id: i64,
	) -> Result<StartGameInfo, UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		// verify access token
		let account_model = verify_access_token(&tx, access_token).await?;
		let uid = account_model.uid;

		// find profile
		let profile_model = profile::Entity::find()
			.filter(profile::Column::AccountId.eq(uid))
			.filter(profile::Column::Id.eq(profile_id))
			.one(&tx)
			.await?;

		let Some(profile_model) = profile_model else {
			return Err(UserError::ProfileNotFound);
		};

		let token = issue_token(&tx, uid, profile_id, TokenType::Session).await?;

		tx.commit().await?;

		Ok(StartGameInfo {
			profile: profile_model.into(),
			session: token,
		})
	}

	async fn select_world(&self, profile_id: i64, world_id: i64) -> Result<(), UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		let Some(profile_model) = profile::Entity::find_by_id(profile_id).one(&tx).await? else {
			return Err(UserError::ProfileNotFound);
		};

		let mut am: profile::ActiveModel = profile_model.into();
		am.world_id = ActiveValue::Set(world_id);
		am.update(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn find_profile(&self, profile_id: i64) -> Result<Profile, UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		let Some(profile_model) = profile::Entity::find_by_id(profile_id).one(&tx).await? else {
			return Err(UserError::ProfileNotFound);
		};

		tx.commit().await?;

		Ok(profile_model.into())
	}

	async fn delete_profile(&self, _access_token: &str) -> Result<(), UserError> {
		// TODO: implement
		Ok(())
	}
}
