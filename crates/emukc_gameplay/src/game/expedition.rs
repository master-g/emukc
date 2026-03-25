use async_trait::async_trait;
use emukc_db::{
	entity::profile::{self, expedition, fleet, item::slot_item, ship},
	sea_orm::{ActiveValue, IntoActiveModel, QueryFilter, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcUseItemType, MaterialCategory, level, start2::ApiMstMission},
	thirdparty::{
		ExpeditionResult, Kc3rdCompositionAlternative, Kc3rdExpeditionCondition,
		Kc3rdShipTypeRequirement, QuestActionEvent,
	},
};
use emukc_time::{
	KcTime,
	chrono::{DateTime, Datelike, Duration, FixedOffset, TimeZone, Utc},
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	basic::find_profile,
	fleet::{find_fleet, get_fleet_ships_impl},
	material::add_material_impl,
	quest::update::update_quest_progress_for_action,
	ship::recalculate_ship_status_with_model,
	use_item::add_use_item_impl,
};

const DRUM_CANISTER_MST_ID: i64 = 75;

#[derive(Debug, Clone)]
pub struct ExpeditionStartInfo {
	pub complete_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExpeditionItemReward {
	pub item_id: i64,
	pub count: i64,
	pub name: String,
}

#[derive(Debug, Clone)]
pub struct ExpeditionCompletion {
	pub mission_id: i64,
	pub fleet_id: i64,
	pub result: ExpeditionResult,
	pub ship_ids: Vec<i64>,
	pub admiral_exp: i64,
	pub member_lv: i64,
	pub member_exp: i64,
	pub ship_exp: Vec<i64>,
	pub ship_exp_after: Vec<[i64; 2]>,
	pub maparea_name: String,
	pub detail: String,
	pub quest_name: String,
	pub quest_level: i64,
	pub resource_reward: Option<[i64; 4]>,
	pub item_rewards: Vec<ExpeditionItemReward>,
}

/// A trait for expedition(mission) related gameplay.
#[async_trait]
pub trait ExpeditionOps {
	/// Get all expedition records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_expeditions(
		&self,
		profile_id: i64,
	) -> Result<(Vec<expedition::Model>, Option<i64>), GameplayError>;

	/// Start an expedition for a fleet.
	async fn start_expedition(
		&self,
		profile_id: i64,
		fleet_id: i64,
		mission_id: i64,
	) -> Result<ExpeditionStartInfo, GameplayError>;

	/// Complete an expedition and receive its rewards.
	async fn complete_expedition(
		&self,
		profile_id: i64,
		fleet_id: i64,
	) -> Result<ExpeditionCompletion, GameplayError>;

	/// Recall an expedition currently in progress.
	async fn recall_expedition(&self, profile_id: i64, fleet_id: i64) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ExpeditionOps for T {
	async fn get_expeditions(
		&self,
		profile_id: i64,
	) -> Result<(Vec<expedition::Model>, Option<i64>), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let records = expedition::Entity::find()
			.filter(expedition::Column::ProfileId.eq(profile_id))
			.all(&tx)
			.await?;

		let now = Utc::now();
		let next_refresh_time = codex
			.manifest
			.api_mst_mission
			.iter()
			.any(|mission| mission.api_reset_type == 1)
			.then(|| next_monthly_reset_time(now).timestamp());

		let mut result: Vec<expedition::Model> = Vec::with_capacity(records.len());
		for record in records {
			let mission_mst = find_mission_mst(codex, record.mission_id).ok_or_else(|| {
				GameplayError::BadManifest(format!("mission {} not found", record.mission_id))
			})?;
			result.push(refresh_monthly_record(&tx, record, mission_mst, now).await?);
		}

		tx.commit().await?;

		Ok((result, next_refresh_time))
	}

	async fn start_expedition(
		&self,
		profile_id: i64,
		fleet_id: i64,
		mission_id: i64,
	) -> Result<ExpeditionStartInfo, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let mission_mst = find_mission_mst(codex, mission_id)
			.ok_or(GameplayError::ManifestNotFound(mission_id))?;
		let expedition_condition =
			codex.expedition_conditions.get(&mission_id).ok_or_else(|| {
				GameplayError::BadManifest(format!("expedition condition {mission_id} not found"))
			})?;

		let fleet_model = find_fleet(&tx, profile_id, fleet_id).await?;
		if fleet_model.mission_status != fleet::MissionStatus::Idle {
			return Err(GameplayError::WrongType(format!(
				"fleet {fleet_id} is already on mission status {:?}",
				fleet_model.mission_status,
			)));
		}

		let fleet_ships = get_fleet_ships_impl(&tx, profile_id, fleet_id).await?;
		validate_expedition_start(&tx, codex, &fleet_ships, expedition_condition).await?;

		let complete_time = Utc::now() + Duration::minutes(mission_mst.api_time);

		{
			let mut am = fleet_model.into_active_model();
			am.mission_status = ActiveValue::Set(fleet::MissionStatus::InMission);
			am.mission_id = ActiveValue::Set(mission_id);
			am.return_time = ActiveValue::Set(Some(complete_time));
			am.update(&tx).await?;
		}

		mark_expedition_started(&tx, profile_id, mission_mst, Utc::now()).await?;

		tx.commit().await?;

		Ok(ExpeditionStartInfo {
			complete_time,
		})
	}

	async fn complete_expedition(
		&self,
		profile_id: i64,
		fleet_id: i64,
	) -> Result<ExpeditionCompletion, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let fleet_model = find_fleet(&tx, profile_id, fleet_id).await?;
		if fleet_model.mission_status == fleet::MissionStatus::Idle || fleet_model.mission_id <= 0 {
			return Err(GameplayError::WrongType(
				format!("fleet {fleet_id} is not on expedition",),
			));
		}

		let now = Utc::now();
		let result = match fleet_model.mission_status {
			fleet::MissionStatus::ForceReturning => ExpeditionResult::Failure,
			fleet::MissionStatus::Returning => ExpeditionResult::Success,
			fleet::MissionStatus::InMission => {
				let return_time = fleet_model.return_time.ok_or_else(|| {
					GameplayError::WrongType(format!(
						"fleet {fleet_id} is missing expedition return time",
					))
				})?;
				if return_time > now {
					return Err(GameplayError::WrongType(format!(
						"fleet {fleet_id} expedition {} is not ready yet",
						fleet_model.mission_id,
					)));
				}
				ExpeditionResult::Success
			}
			fleet::MissionStatus::Idle => unreachable!(),
		};

		let mission_id = fleet_model.mission_id;
		let mission_mst = find_mission_mst(codex, mission_id)
			.ok_or(GameplayError::ManifestNotFound(mission_id))?;
		let expedition_condition =
			codex.expedition_conditions.get(&mission_id).ok_or_else(|| {
				GameplayError::BadManifest(format!("expedition condition {mission_id} not found"))
			})?;
		let mission_ships = get_fleet_ships_impl(&tx, profile_id, fleet_id).await?;
		let ship_ids: Vec<i64> = mission_ships.iter().map(|ship| ship.id).collect();

		let (profile, ship_exp, ship_exp_after, resource_reward, item_rewards) =
			if result == ExpeditionResult::Success || result == ExpeditionResult::GreatSuccess {
				let item_rewards =
					grant_expedition_rewards(&tx, codex, profile_id, expedition_condition).await?;
				let profile = apply_profile_expedition_result(
					&tx,
					profile_id,
					expedition_condition.admiral_exp,
					true,
				)
				.await?;
				let (ship_exp, ship_exp_after) = apply_ship_expedition_exp(
					&tx,
					codex,
					&mission_ships,
					expedition_condition.fleet_exp,
				)
				.await?;
				mark_expedition_completed(&tx, profile_id, mission_id, now).await?;

				let event = QuestActionEvent::ExpeditionCompleted {
					mission_id,
					result,
					fleet_id,
				};
				update_quest_progress_for_action(&tx, codex, profile_id, &event).await?;

				(
					profile,
					ship_exp,
					ship_exp_after,
					Some(expedition_condition.resource_reward),
					item_rewards,
				)
			} else {
				let profile = apply_profile_expedition_result(&tx, profile_id, 0, false).await?;
				let ship_exp = vec![0; mission_ships.len()];
				let ship_exp_after = mission_ships
					.iter()
					.map(|ship| {
						[
							ship.exp_now,
							if ship.exp_next > 0 {
								ship.exp_next
							} else {
								-1
							},
						]
					})
					.collect();
				(profile, ship_exp, ship_exp_after, None, vec![])
			};

		reset_fleet_expedition(&tx, fleet_model).await?;
		tx.commit().await?;

		let maparea_name = codex
			.manifest
			.api_mst_maparea
			.iter()
			.find(|area| area.api_id == mission_mst.api_maparea_id)
			.map(|area| area.api_name.clone())
			.unwrap_or_default();

		Ok(ExpeditionCompletion {
			mission_id,
			fleet_id,
			result,
			ship_ids,
			admiral_exp: if result == ExpeditionResult::Success
				|| result == ExpeditionResult::GreatSuccess
			{
				expedition_condition.admiral_exp
			} else {
				0
			},
			member_lv: profile.hq_level,
			member_exp: profile.experience,
			ship_exp,
			ship_exp_after,
			maparea_name,
			detail: mission_mst.api_details.clone(),
			quest_name: mission_mst.api_name.clone(),
			quest_level: mission_mst.api_difficulty,
			resource_reward,
			item_rewards,
		})
	}

	async fn recall_expedition(&self, profile_id: i64, fleet_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let fleet_model = find_fleet(&tx, profile_id, fleet_id).await?;
		if fleet_model.mission_status != fleet::MissionStatus::InMission {
			return Err(GameplayError::WrongType(
				format!("fleet {fleet_id} is not in expedition",),
			));
		}

		let mut am = fleet_model.into_active_model();
		am.mission_status = ActiveValue::Set(fleet::MissionStatus::ForceReturning);
		am.return_time = ActiveValue::Set(Some(Utc::now()));
		am.update(&tx).await?;

		tx.commit().await?;

		Ok(())
	}
}

async fn validate_expedition_start<C>(
	c: &C,
	codex: &Codex,
	fleet_ships: &[ship::Model],
	expedition_condition: &Kc3rdExpeditionCondition,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	if fleet_ships.is_empty() {
		return Err(GameplayError::WrongType("fleet is empty".to_string()));
	}

	let requirements = &expedition_condition.requirements;
	if fleet_ships.len() < requirements.ship_count as usize {
		return Err(GameplayError::WrongType(format!(
			"fleet requires at least {} ships for expedition {}",
			requirements.ship_count, expedition_condition.api_id,
		)));
	}

	let flagship = fleet_ships.first().unwrap();
	let flagship_mst = codex
		.manifest
		.find_ship(flagship.mst_id)
		.ok_or(GameplayError::ManifestNotFound(flagship.mst_id))?;

	if requirements.flagship_level.is_some_and(|min| flagship.level < min) {
		return Err(GameplayError::WrongType(format!(
			"flagship level {} is lower than required {}",
			flagship.level,
			requirements.flagship_level.unwrap_or_default(),
		)));
	}

	if requirements.flagship_type.is_some_and(|stype| flagship_mst.api_stype != stype) {
		return Err(GameplayError::WrongType(format!(
			"flagship type {} does not satisfy required {}",
			flagship_mst.api_stype,
			requirements.flagship_type.unwrap_or_default(),
		)));
	}

	if requirements
		.fleet_level
		.is_some_and(|min_total| fleet_ships.iter().map(|ship| ship.level).sum::<i64>() < min_total)
	{
		return Err(GameplayError::WrongType(format!(
			"fleet total level is lower than required {}",
			requirements.fleet_level.unwrap_or_default(),
		)));
	}

	if requirements.total_firepower.is_some_and(|min_total| {
		fleet_ships.iter().map(|ship| ship.firepower_now).sum::<i64>() < min_total
	}) {
		return Err(GameplayError::WrongType(format!(
			"fleet total firepower is lower than required {}",
			requirements.total_firepower.unwrap_or_default(),
		)));
	}

	if requirements.total_asw.is_some_and(|min_total| {
		fleet_ships.iter().map(|ship| ship.asw_now).sum::<i64>() < min_total
	}) {
		return Err(GameplayError::WrongType(format!(
			"fleet total ASW is lower than required {}",
			requirements.total_asw.unwrap_or_default(),
		)));
	}

	if requirements.total_los.is_some_and(|min_total| {
		fleet_ships.iter().map(|ship| ship.los_now).sum::<i64>() < min_total
	}) {
		return Err(GameplayError::WrongType(format!(
			"fleet total LOS is lower than required {}",
			requirements.total_los.unwrap_or_default(),
		)));
	}

	if let Some(drum) = &requirements.drum_requirements {
		let (ships_with_drums, total_drums) = count_drum_canisters(c, fleet_ships).await?;
		let ship_count_ok = drum.optional || ships_with_drums >= drum.ship_count;
		if !ship_count_ok || total_drums < drum.total_count {
			return Err(GameplayError::WrongType(format!(
				"fleet drum canister requirement not met: ships={}, total={}",
				ships_with_drums, total_drums,
			)));
		}
	}

	if !requirements.composition.is_empty()
		&& !requirements
			.composition
			.iter()
			.any(|alt| expedition_alternative_matches(codex, fleet_ships, alt))
	{
		return Err(GameplayError::WrongType(format!(
			"fleet composition does not satisfy expedition {}",
			expedition_condition.api_id,
		)));
	}

	Ok(())
}

fn expedition_alternative_matches(
	codex: &Codex,
	fleet_ships: &[ship::Model],
	alternative: &Kc3rdCompositionAlternative,
) -> bool {
	alternative
		.conditions
		.iter()
		.all(|condition| expedition_ship_type_requirement_matches(codex, fleet_ships, condition))
}

fn expedition_ship_type_requirement_matches(
	codex: &Codex,
	fleet_ships: &[ship::Model],
	condition: &Kc3rdShipTypeRequirement,
) -> bool {
	let count = fleet_ships
		.iter()
		.filter(|ship| {
			codex.manifest.find_ship(ship.mst_id).is_some_and(|mst| {
				condition.ship_types.iter().any(|ship_type| *ship_type == mst.api_stype)
			})
		})
		.count() as i64;

	count >= condition.count
}

async fn count_drum_canisters<C>(
	c: &C,
	fleet_ships: &[ship::Model],
) -> Result<(i64, i64), GameplayError>
where
	C: ConnectionTrait,
{
	let slot_ids: Vec<i64> = fleet_ships
		.iter()
		.flat_map(|ship| {
			[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
		})
		.filter(|slot_id| *slot_id > 0)
		.collect();

	if slot_ids.is_empty() {
		return Ok((0, 0));
	}

	let slot_items =
		slot_item::Entity::find().filter(slot_item::Column::Id.is_in(slot_ids)).all(c).await?;

	let mut ships_with_drums = 0;
	let mut total_drums = 0;
	for ship in fleet_ships {
		let ship_slot_ids =
			[ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex];
		let drum_count = slot_items
			.iter()
			.filter(|item| ship_slot_ids.contains(&item.id) && item.mst_id == DRUM_CANISTER_MST_ID)
			.count() as i64;
		if drum_count > 0 {
			ships_with_drums += 1;
		}
		total_drums += drum_count;
	}

	Ok((ships_with_drums, total_drums))
}

fn find_mission_mst(codex: &Codex, mission_id: i64) -> Option<&ApiMstMission> {
	codex.manifest.api_mst_mission.iter().find(|mission| mission.api_id == mission_id)
}

async fn mark_expedition_started<C>(
	c: &C,
	profile_id: i64,
	mission_mst: &ApiMstMission,
	now: DateTime<Utc>,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let existing = expedition::Entity::find()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.filter(expedition::Column::MissionId.eq(mission_mst.api_id))
		.one(c)
		.await?;

	match existing {
		Some(record) => {
			let record = refresh_monthly_record(c, record, mission_mst, now).await?;
			if record.state != expedition::Status::Completed {
				let mut am = record.into_active_model();
				am.state = ActiveValue::Set(expedition::Status::Unfinished);
				am.update(c).await?;
			}
		}
		None => {
			expedition::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				mission_id: ActiveValue::Set(mission_mst.api_id),
				state: ActiveValue::Set(expedition::Status::Unfinished),
				last_completed_at: ActiveValue::Set(None),
			}
			.insert(c)
			.await?;
		}
	}

	Ok(())
}

async fn mark_expedition_completed<C>(
	c: &C,
	profile_id: i64,
	mission_id: i64,
	completed_at: DateTime<Utc>,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let existing = expedition::Entity::find()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.filter(expedition::Column::MissionId.eq(mission_id))
		.one(c)
		.await?;

	match existing {
		Some(record) => {
			let mut am = record.into_active_model();
			am.state = ActiveValue::Set(expedition::Status::Completed);
			am.last_completed_at = ActiveValue::Set(Some(completed_at));
			am.update(c).await?;
		}
		None => {
			expedition::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				mission_id: ActiveValue::Set(mission_id),
				state: ActiveValue::Set(expedition::Status::Completed),
				last_completed_at: ActiveValue::Set(Some(completed_at)),
			}
			.insert(c)
			.await?;
		}
	}

	Ok(())
}

async fn reset_fleet_expedition<C>(
	c: &C,
	model: fleet::Model,
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let mut am = model.into_active_model();
	am.mission_status = ActiveValue::Set(fleet::MissionStatus::Idle);
	am.mission_id = ActiveValue::Set(0);
	am.return_time = ActiveValue::Set(None);

	Ok(am.update(c).await?)
}

async fn apply_profile_expedition_result<C>(
	c: &C,
	profile_id: i64,
	admiral_exp: i64,
	success: bool,
) -> Result<profile::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = find_profile(c, profile_id).await?;
	let mut am = record.into_active_model();

	let current_exp = am.experience.clone().take().unwrap_or_default();
	let new_exp = current_exp + admiral_exp;
	let (hq_level, _) = level::exp_to_hq_level(new_exp);
	let expedition_count = am.expeditions.clone().take().unwrap_or_default() + 1;
	let expedition_success = am.expeditions_success.clone().take().unwrap_or_default()
		+ if success {
			1
		} else {
			0
		};

	am.experience = ActiveValue::Set(new_exp);
	am.hq_level = ActiveValue::Set(hq_level);
	am.expeditions = ActiveValue::Set(expedition_count);
	am.expeditions_success = ActiveValue::Set(expedition_success);

	Ok(am.update(c).await?)
}

async fn apply_ship_expedition_exp<C>(
	c: &C,
	codex: &Codex,
	ships: &[ship::Model],
	fleet_exp: i64,
) -> Result<(Vec<i64>, Vec<[i64; 2]>), GameplayError>
where
	C: ConnectionTrait,
{
	let mut gains = Vec::with_capacity(ships.len());
	let mut after = Vec::with_capacity(ships.len());

	for model in ships {
		let gain = fleet_exp.max(0);
		gains.push(gain);

		let mut updated = model.clone();
		updated.exp_now += gain;

		let (level, next_exp) = level::exp_to_ship_level(updated.exp_now);
		let current_level_exp = level::ship_level_required_exp(level);
		let progress = if next_exp > current_level_exp {
			((updated.exp_now - current_level_exp) * 100 / (next_exp - current_level_exp))
				.clamp(0, 99)
		} else {
			0
		};

		updated.level = level;
		updated.exp_next = next_exp;
		updated.exp_progress = progress;

		let persisted =
			recalculate_ship_status_with_model(c, codex, &updated).await?.update(c).await?;
		after.push([
			persisted.exp_now,
			if persisted.exp_next > 0 {
				persisted.exp_next
			} else {
				-1
			},
		]);
	}

	Ok((gains, after))
}

async fn grant_expedition_rewards<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	expedition_condition: &Kc3rdExpeditionCondition,
) -> Result<Vec<ExpeditionItemReward>, GameplayError>
where
	C: ConnectionTrait,
{
	let mats = [
		(MaterialCategory::Fuel, expedition_condition.resource_reward[0]),
		(MaterialCategory::Ammo, expedition_condition.resource_reward[1]),
		(MaterialCategory::Steel, expedition_condition.resource_reward[2]),
		(MaterialCategory::Bauxite, expedition_condition.resource_reward[3]),
	]
	.into_iter()
	.filter(|(_, amount)| *amount > 0)
	.collect::<Vec<_>>();

	if !mats.is_empty() {
		add_material_impl(c, codex, profile_id, &mats).await?;
	}

	let mut items = Vec::with_capacity(expedition_condition.item_rewards.len());
	for reward in &expedition_condition.item_rewards {
		match KcUseItemType::n(reward.item_id) {
			Some(KcUseItemType::Bucket) => {
				add_material_impl(
					c,
					codex,
					profile_id,
					&[(MaterialCategory::Bucket, reward.count)],
				)
				.await?;
			}
			Some(KcUseItemType::Torch) => {
				add_material_impl(c, codex, profile_id, &[(MaterialCategory::Torch, reward.count)])
					.await?;
			}
			Some(KcUseItemType::DevMaterial) => {
				add_material_impl(
					c,
					codex,
					profile_id,
					&[(MaterialCategory::DevMat, reward.count)],
				)
				.await?;
			}
			Some(KcUseItemType::Screw) => {
				add_material_impl(c, codex, profile_id, &[(MaterialCategory::Screw, reward.count)])
					.await?;
			}
			Some(KcUseItemType::Fuel) => {
				add_material_impl(c, codex, profile_id, &[(MaterialCategory::Fuel, reward.count)])
					.await?;
			}
			Some(KcUseItemType::Ammo) => {
				add_material_impl(c, codex, profile_id, &[(MaterialCategory::Ammo, reward.count)])
					.await?;
			}
			Some(KcUseItemType::Steel) => {
				add_material_impl(c, codex, profile_id, &[(MaterialCategory::Steel, reward.count)])
					.await?;
			}
			Some(KcUseItemType::Bauxite) => {
				add_material_impl(
					c,
					codex,
					profile_id,
					&[(MaterialCategory::Bauxite, reward.count)],
				)
				.await?;
			}
			_ => {
				add_use_item_impl(c, profile_id, reward.item_id, reward.count).await?;
			}
		}

		let name = codex
			.manifest
			.find_useitem(reward.item_id)
			.map(|item| item.api_name.clone())
			.unwrap_or_else(|| reward.item_id.to_string());
		items.push(ExpeditionItemReward {
			item_id: reward.item_id,
			count: reward.count,
			name,
		});
	}

	Ok(items)
}

async fn refresh_monthly_record<C>(
	c: &C,
	record: expedition::Model,
	mission_mst: &ApiMstMission,
	now: DateTime<Utc>,
) -> Result<expedition::Model, GameplayError>
where
	C: ConnectionTrait,
{
	if mission_mst.api_reset_type != 1 || record.state != expedition::Status::Completed {
		return Ok(record);
	}

	let first_day_of_this_month = KcTime::jst_0500_of_nth_day(1);
	if now <= first_day_of_this_month {
		return Ok(record);
	}

	if record.last_completed_at.is_none_or(|completed_at| completed_at >= first_day_of_this_month) {
		return Ok(record);
	}

	let mut am = record.into_active_model();
	am.state = ActiveValue::Set(expedition::Status::NotStarted);
	am.last_completed_at = ActiveValue::Set(None);

	Ok(am.update(c).await?)
}

fn next_monthly_reset_time(now: DateTime<Utc>) -> DateTime<Utc> {
	let tokyo_tz = FixedOffset::east_opt(9 * 3600).unwrap();
	let now_jst = now.with_timezone(&tokyo_tz);
	let (year, month) = if now_jst.month() == 12 {
		(now_jst.year() + 1, 1)
	} else {
		(now_jst.year(), now_jst.month() + 1)
	};

	tokyo_tz.with_ymd_and_hms(year, month, 1, 5, 0, 0).single().unwrap().with_timezone(&Utc)
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
	expedition::Entity::delete_many()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
