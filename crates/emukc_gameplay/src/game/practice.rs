use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		self,
		practice::{self, config::RivalType, ship},
	},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait},
};
use emukc_model::{
	codex::Codex,
	kc2::UserHQRank,
	profile::practice::{PracticeConfig, Rival, RivalDetail, RivalFlag, RivalShip, RivalStatus},
};
use emukc_time::{
	chrono::{DateTime, Duration, Utc},
	KcTime,
};
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{err::GameplayError, gameplay::HasContext};

/// Practice information.
#[derive(Debug, Clone)]
pub struct PracticeInfo {
	/// The practice rivals.
	pub rivals: Vec<Rival>,

	/// current practice config.
	pub cfg: practice::config::Model,

	/// The entry limit.
	pub entry_limit: Option<i64>,
}

/// A trait for practice related gameplay.
#[async_trait]
pub trait PracticeOps {
	/// Get practice rivals.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_practice_rivals(&self, profile_id: i64) -> Result<PracticeInfo, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PracticeOps for T {
	async fn get_practice_rivals(&self, profile_id: i64) -> Result<PracticeInfo, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let rivals = get_practice_rivals_impl(&tx, codex, profile_id).await?;

		tx.commit().await?;

		Ok(rivals)
	}
}

pub(crate) async fn get_practice_rivals_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<PracticeInfo, GameplayError>
where
	C: ConnectionTrait,
{
	let config = profile::practice::config::Entity::find_by_id(profile_id)
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"Practice config not found for profile {}",
				profile_id
			))
		})?;

	let rivals = profile::practice::rival::Entity::find()
		.filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
		.order_by_asc(profile::practice::rival::Column::Index)
		.all(c)
		.await?;

	let entry_limit = cal_practice_entry_limit();

	let resp = if rivals.is_empty()
		|| KcTime::is_before_or_after_jst_today_hour(&config.last_generated, 3, 15)
	{
		let selected_type = config.selected_type;
		let r = generate_practice_rivals(c, codex, profile_id, selected_type).await?;

		// update last generated time
		let mut cfg_am = config.into_active_model();
		cfg_am.last_generated = ActiveValue::Set(Utc::now());
		cfg_am.generated_type = ActiveValue::Set(selected_type);

		let cfg = cfg_am.update(c).await?;

		PracticeInfo {
			rivals: r,
			cfg,
			entry_limit,
		}
	} else {
		let mut r: Vec<Rival> = Vec::new();

		for rival in rivals.into_iter() {
			// find rival details and all ships
			let details = profile::practice::detail::Entity::find_by_id(rival.id)
				.one(c)
				.await?
				.ok_or_else(|| {
					GameplayError::EntryNotFound(format!(
						"Practice rival details not found for profile {}",
						profile_id
					))
				})?;

			let ships = profile::practice::ship::Entity::find()
				.filter(profile::practice::ship::Column::ProfileId.eq(profile_id))
				.filter(profile::practice::ship::Column::RivalId.eq(rival.id))
				.all(c)
				.await?;

			r.push(Rival {
				id: rival.id,
				index: rival.index,
				name: rival.name,
				comment: rival.comment,
				level: rival.level,
				rank: UserHQRank::n(rival.rank).unwrap(),
				flag: rival.flag.into(),
				status: rival.status.into(),
				medals: rival.medals,
				details: RivalDetail {
					exp_now: details.exp_now,
					exp_next: details.exp_next,
					friend: details.friend,
					current_ship_count: details.current_ship_count,
					ship_capacity: details.ship_capacity,
					current_slot_item_count: details.current_slot_item_count,
					slot_item_capacity: details.slot_item_capacity,
					furniture: details.furniture,
					deck_name: details.deck_name,
					ships: ships
						.into_iter()
						.map(|s| RivalShip {
							id: s.id,
							mst_id: s.mst_id,
							level: s.level,
							star: s.star,
						})
						.collect(),
				},
			});
		}

		PracticeInfo {
			rivals: r,
			cfg: config,
			entry_limit,
		}
	};

	Ok(resp)
}

/// Generate practice rivals.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `codex`: The codex.
/// - `profile_id`: The profile ID.
/// - `select_type`: The rival type.
///
/// ## TODO
/// 1. using normal distribution to generate 5 rivals
/// 2. using player's level as the mean of the distribution
///
/// there are a few types of rivals' deck:
///
/// a. best formation, tier S ships, since they are farming event or boss.
/// b. carefully hand picked ships to irradiate you, very difficult to beat.
/// c. 1 or 2 high level ships, they want to help you to level up.
async fn generate_practice_rivals<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	_select_type: RivalType,
) -> Result<Vec<Rival>, GameplayError>
where
	C: ConnectionTrait,
{
	let mut api_mst_ship = codex.manifest.api_mst_ship.clone();
	api_mst_ship.retain(|f| f.api_aftershipid.is_some());

	let last_profile = profile::Entity::find().order_by_desc(profile::Column::Id).one(c).await?;
	let current_uid = last_profile.map(|p| p.id).unwrap_or(0) + 1;

	let last_ship = ship::Entity::find().order_by_desc(ship::Column::Id).one(c).await?;
	let current_ship_id = last_ship.map(|s| s.id).unwrap_or(0) + 1;

	let rival_uid_starts_from = current_uid + 10000;
	let rival_ship_id_starts_from = current_ship_id + 10000;

	let mut r = SmallRng::from_entropy();

	let rivals: Vec<Rival> = (1..6)
		.map(|i| {
			let name = format!("Practice Rival {}", i);
			let comment = format!("I am the {}th rival", i);
			let rank = r.gen_range(1..=10);
			let flag = r.gen_range(1..=3);
			let ship_mst = api_mst_ship.choose(&mut r).unwrap();

			Rival {
				id: rival_uid_starts_from + i,
				index: i,
				name,
				comment,
				level: 120 - i,
				rank: UserHQRank::n(rank).unwrap_or(UserHQRank::Admiral),
				flag: RivalFlag::n(flag).unwrap_or_default(),
				status: RivalStatus::Untouched,
				medals: 10 + i,
				details: RivalDetail {
					exp_now: 0,
					exp_next: 1000,
					friend: 0,
					current_ship_count: 299,
					ship_capacity: 300,
					current_slot_item_count: 999,
					slot_item_capacity: 1000,
					furniture: 123,
					deck_name: format!("Deck {}", i),
					ships: vec![RivalShip {
						id: rival_ship_id_starts_from + i,
						mst_id: ship_mst.api_id,
						level: 180,
						star: 10,
					}],
				},
			}
		})
		.collect();

	// remove old records
	profile::practice::rival::Entity::delete_many()
		.filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	profile::practice::detail::Entity::delete_many()
		.filter(profile::practice::detail::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	profile::practice::ship::Entity::delete_many()
		.filter(profile::practice::ship::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	// insert new records
	for (i, rival) in rivals.iter().enumerate() {
		// rival
		{
			let am = profile::practice::rival::ActiveModel {
				id: ActiveValue::Set(rival.id),
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(i as i64 + 1),
				name: ActiveValue::Set(rival.name.clone()),
				comment: ActiveValue::Set(rival.comment.clone()),
				level: ActiveValue::Set(rival.level),
				rank: ActiveValue::Set(rival.rank as i64),
				flag: ActiveValue::Set(rival.flag.into()),
				status: ActiveValue::Set(rival.status.into()),
				medals: ActiveValue::Set(rival.medals),
			};

			am.insert(c).await?;
		}
		// details
		{
			let am = profile::practice::detail::ActiveModel {
				id: ActiveValue::Set(rival.id),
				profile_id: ActiveValue::Set(profile_id),
				exp_now: ActiveValue::Set(rival.details.exp_now),
				exp_next: ActiveValue::Set(rival.details.exp_next),
				friend: ActiveValue::Set(rival.details.friend),
				current_ship_count: ActiveValue::Set(rival.details.current_ship_count),
				ship_capacity: ActiveValue::Set(rival.details.ship_capacity),
				current_slot_item_count: ActiveValue::Set(rival.details.current_slot_item_count),
				slot_item_capacity: ActiveValue::Set(rival.details.slot_item_capacity),
				furniture: ActiveValue::Set(rival.details.furniture),
				deck_name: ActiveValue::Set(rival.details.deck_name.clone()),
			};

			am.insert(c).await?;
		}
		// ships
		{
			for ship in &rival.details.ships {
				let am = profile::practice::ship::ActiveModel {
					id: ActiveValue::Set(ship.id),
					profile_id: ActiveValue::Set(profile_id),
					rival_id: ActiveValue::Set(rival.id),
					mst_id: ActiveValue::Set(ship.mst_id),
					level: ActiveValue::Set(ship.level),
					star: ActiveValue::Set(ship.star),
				};

				am.insert(c).await?;
			}
		}
	}

	Ok(rivals)
}

fn cal_practice_entry_limit() -> Option<i64> {
	let now = Utc::now();
	let jst_0100 = KcTime::jst_today_hour_utc(1);

	if now < jst_0100 {
		return None;
	}
	let jst_1300 = KcTime::jst_today_hour_utc(13);
	if now < jst_1300 {
		return Some((jst_1300.timestamp_millis() - now.timestamp_millis()) / 1000);
	}

	let jst_next_0100 = jst_0100 + Duration::days(1);
	let diff = jst_next_0100.timestamp_millis() - now.timestamp_millis();
	if diff > 0 {
		Some(diff / 1000)
	} else {
		None
	}
}

/// Initialize practice of user.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(
	c: &C,
	profile_id: i64,
) -> Result<profile::practice::config::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let config = PracticeConfig::default();
	let am: profile::practice::config::ActiveModel = profile::practice::config::ActiveModel {
		id: ActiveValue::Set(profile_id),
		selected_type: ActiveValue::Set(config.selected_type.into()),
		generated_type: ActiveValue::Set(config.generated_type.into()),
		last_generated: ActiveValue::Set(DateTime::UNIX_EPOCH),
	};

	let m = am.insert(c).await?;

	Ok(m)
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	profile::practice::config::Entity::delete_by_id(profile_id).exec(c).await?;
	profile::practice::rival::Entity::delete_many()
		.filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	profile::practice::detail::Entity::delete_many()
		.filter(profile::practice::detail::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	profile::practice::ship::Entity::delete_many()
		.filter(profile::practice::ship::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
