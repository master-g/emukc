use async_trait::async_trait;

use emukc_crypto::rng;
use emukc_db::{
    entity::profile::{self, expedition, fleet, item::slot_item, ship},
    sea_orm::{ActiveValue, IntoActiveModel, QueryFilter, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::{KcUseItemType, MaterialCategory, level, start2::ApiMstMission},
    prelude::ApiMstShip,
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
    ship::{get_ships_impl, recalculate_ship_status_with_model},
    use_item::add_use_item_impl,
};

const DRUM_CANISTER_MST_ID: i64 = 75;

#[derive(Debug, Clone)]
pub struct ExpeditionStartInfo {
    pub complete_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub item_rewards: [Option<ExpeditionItemReward>; 2],
}

#[derive(Debug, Clone, Copy)]
struct ExpeditionSupplyCost {
    ship_id: i64,
    fuel: i64,
    ammo: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GreatSuccessType {
    Type1,
    Type2,
    Type3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpeditionFailureKind {
    Recall,
    Fatigue,
}

#[derive(Debug, Clone, Copy)]
struct ExpeditionLaunchSnapshot {
    fleet_ship_count: i64,
    sparkled_ship_count: i64,
    flagship_level: i64,
    drum_ship_count: i64,
    total_drums: i64,
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
        let launch_snapshot = collect_expedition_launch_snapshot(&tx, &fleet_ships).await?;
        let supply_costs = calculate_expedition_supply_costs(codex, mission_mst, &fleet_ships)?;
        apply_expedition_supply_costs(&tx, &fleet_ships, &supply_costs).await?;

        let complete_time = Utc::now() + Duration::minutes(mission_mst.api_time);

        {
            let mut am = fleet_model.into_active_model();
            am.mission_status = ActiveValue::Set(fleet::MissionStatus::InMission);
            am.mission_id = ActiveValue::Set(mission_id);
            am.return_time = ActiveValue::Set(Some(complete_time));
            am.launch_fleet_ship_count = ActiveValue::Set(launch_snapshot.fleet_ship_count);
            am.launch_sparkled_ship_count = ActiveValue::Set(launch_snapshot.sparkled_ship_count);
            am.launch_flagship_level = ActiveValue::Set(launch_snapshot.flagship_level);
            am.launch_drum_ship_count = ActiveValue::Set(launch_snapshot.drum_ship_count);
            am.launch_total_drums = ActiveValue::Set(launch_snapshot.total_drums);
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
        let mission_id = fleet_model.mission_id;
        let mission_mst = find_mission_mst(codex, mission_id)
            .ok_or(GameplayError::ManifestNotFound(mission_id))?;
        let expedition_condition =
            codex.expedition_conditions.get(&mission_id).ok_or_else(|| {
                GameplayError::BadManifest(format!("expedition condition {mission_id} not found"))
            })?;
        let mission_ships_before_return =
            get_fleet_ships_with_morale_refresh(&tx, profile_id, fleet_id).await?;
        let ship_ids: Vec<i64> = mission_ships_before_return.iter().map(|ship| ship.id).collect();
        let is_recall = fleet_model.mission_status == fleet::MissionStatus::ForceReturning;
        let launch_snapshot = load_expedition_launch_snapshot(&fleet_model).unwrap_or(
            collect_expedition_launch_snapshot(&tx, &mission_ships_before_return).await?,
        );
        let mission_ships = match fleet_model.mission_status {
            fleet::MissionStatus::ForceReturning => {
                let return_time = fleet_model.return_time.ok_or_else(|| {
                    GameplayError::WrongType(format!(
                        "fleet {fleet_id} is missing forced-return time",
                    ))
                })?;
                if return_time > now {
                    return Err(GameplayError::WrongType(format!(
                        "fleet {fleet_id} forced return from expedition {} is not ready yet",
                        fleet_model.mission_id,
                    )));
                }
                apply_expedition_return_morale(&tx, mission_mst, &mission_ships_before_return)
                    .await?
            }
            fleet::MissionStatus::Returning | fleet::MissionStatus::InMission => {
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
                apply_expedition_return_morale(&tx, mission_mst, &mission_ships_before_return)
                    .await?
            }
            fleet::MissionStatus::Idle => {
                return Err(idle_fleet_err(fleet_id, "not on an expedition"));
            }
        };
        let result = match fleet_model.mission_status {
            fleet::MissionStatus::ForceReturning => ExpeditionResult::Failure,
            fleet::MissionStatus::Returning | fleet::MissionStatus::InMission => {
                if mission_ships.iter().any(|ship| ship.condition <= 39) {
                    ExpeditionResult::Failure
                } else {
                    determine_expedition_success_result(expedition_condition, launch_snapshot)
                }
            }
            fleet::MissionStatus::Idle => {
                return Err(idle_fleet_err(fleet_id, "cannot determine expedition result"));
            }
        };
        let failure_kind = match result {
            ExpeditionResult::Failure if is_recall => Some(ExpeditionFailureKind::Recall),
            ExpeditionResult::Failure => Some(ExpeditionFailureKind::Fatigue),
            _ => None,
        };

        let (profile, ship_exp, ship_exp_after, resource_reward, item_rewards) =
            if result == ExpeditionResult::Success || result == ExpeditionResult::GreatSuccess {
                let admiral_exp = calculate_expedition_admiral_exp(expedition_condition, result);
                let resource_reward =
                    calculate_expedition_resource_reward(expedition_condition, result);
                let item_rewards = grant_expedition_rewards(
                    &tx,
                    codex,
                    profile_id,
                    expedition_condition,
                    resource_reward,
                    result,
                )
                .await?;
                let profile =
                    apply_profile_expedition_result(&tx, profile_id, admiral_exp, true).await?;
                let (ship_exp, ship_exp_after) = apply_ship_expedition_exp(
                    &tx,
                    codex,
                    &mission_ships,
                    expedition_condition.fleet_exp,
                    result,
                )
                .await?;
                mark_expedition_completed(&tx, profile_id, mission_id, now).await?;

                let event = QuestActionEvent::ExpeditionCompleted {
                    mission_id,
                    result,
                    fleet_id,
                };
                update_quest_progress_for_action(&tx, codex, profile_id, &event).await?;

                (profile, ship_exp, ship_exp_after, Some(resource_reward), item_rewards)
            } else {
                match failure_kind.unwrap() {
                    ExpeditionFailureKind::Recall => {
                        let profile =
                            apply_profile_expedition_result(&tx, profile_id, 0, false).await?;
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
                        (profile, ship_exp, ship_exp_after, None, [None, None])
                    }
                    ExpeditionFailureKind::Fatigue => {
                        let admiral_exp =
                            calculate_failed_expedition_admiral_exp(expedition_condition);
                        let profile =
                            apply_profile_expedition_result(&tx, profile_id, admiral_exp, false)
                                .await?;
                        let (ship_exp, ship_exp_after) = apply_ship_expedition_exp(
                            &tx,
                            codex,
                            &mission_ships,
                            expedition_condition.fleet_exp,
                            ExpeditionResult::Failure,
                        )
                        .await?;
                        (profile, ship_exp, ship_exp_after, None, [None, None])
                    }
                }
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
            admiral_exp: match failure_kind {
                Some(ExpeditionFailureKind::Fatigue) => {
                    calculate_failed_expedition_admiral_exp(expedition_condition)
                }
                _ if result == ExpeditionResult::Success
                    || result == ExpeditionResult::GreatSuccess =>
                {
                    calculate_expedition_admiral_exp(expedition_condition, result)
                }
                _ => 0,
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

        let mission_mst = find_mission_mst(self.codex(), fleet_model.mission_id)
            .ok_or(GameplayError::ManifestNotFound(fleet_model.mission_id))?;
        let now = Utc::now();
        let current_return_time = fleet_model.return_time.ok_or_else(|| {
            GameplayError::WrongType(format!("fleet {fleet_id} is missing expedition return time",))
        })?;
        let forced_return_time =
            calculate_forced_return_time(now, current_return_time, mission_mst);

        let mut am = fleet_model.into_active_model();
        am.mission_status = ActiveValue::Set(fleet::MissionStatus::ForceReturning);
        am.return_time = ActiveValue::Set(Some(forced_return_time));
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

    validate_expedition_full_supply(codex, fleet_ships)?;

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
            codex
                .manifest
                .find_ship(ship.mst_id)
                .is_some_and(|mst| condition.ship_types.contains(&mst.api_stype))
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

async fn collect_expedition_launch_snapshot<C>(
    c: &C,
    fleet_ships: &[ship::Model],
) -> Result<ExpeditionLaunchSnapshot, GameplayError>
where
    C: ConnectionTrait,
{
    let (drum_ship_count, total_drums) = count_drum_canisters(c, fleet_ships).await?;
    Ok(ExpeditionLaunchSnapshot {
        fleet_ship_count: fleet_ships.len() as i64,
        sparkled_ship_count: sparkled_ship_count(fleet_ships),
        flagship_level: fleet_ships.first().map(|ship| ship.level).unwrap_or_default(),
        drum_ship_count,
        total_drums,
    })
}

fn load_expedition_launch_snapshot(fleet_model: &fleet::Model) -> Option<ExpeditionLaunchSnapshot> {
    (fleet_model.launch_fleet_ship_count > 0).then_some(ExpeditionLaunchSnapshot {
        fleet_ship_count: fleet_model.launch_fleet_ship_count,
        sparkled_ship_count: fleet_model.launch_sparkled_ship_count,
        flagship_level: fleet_model.launch_flagship_level,
        drum_ship_count: fleet_model.launch_drum_ship_count,
        total_drums: fleet_model.launch_total_drums,
    })
}

async fn get_fleet_ships_with_morale_refresh<C>(
    c: &C,
    profile_id: i64,
    fleet_id: i64,
) -> Result<Vec<ship::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let fleet = find_fleet(c, profile_id, fleet_id).await?;
    let fleet_ship_ids =
        [fleet.ship_1, fleet.ship_2, fleet.ship_3, fleet.ship_4, fleet.ship_5, fleet.ship_6];
    let (ships, _) = get_ships_impl(c, profile_id).await?;

    let mut mission_ships =
        ships.into_iter().filter(|ship| fleet_ship_ids.contains(&ship.id)).collect::<Vec<_>>();
    mission_ships.sort_by_key(|ship| {
        fleet_ship_ids.iter().position(|ship_id| *ship_id == ship.id).unwrap_or(usize::MAX)
    });

    Ok(mission_ships)
}

fn find_mission_mst(codex: &Codex, mission_id: i64) -> Option<&ApiMstMission> {
    codex.manifest.api_mst_mission.iter().find(|mission| mission.api_id == mission_id)
}

fn validate_expedition_full_supply(
    codex: &Codex,
    fleet_ships: &[ship::Model],
) -> Result<(), GameplayError> {
    for ship in fleet_ships {
        let mst = codex.find::<ApiMstShip>(&ship.mst_id)?;
        let max_fuel = mst.api_fuel_max.ok_or_else(|| {
            GameplayError::BadManifest(format!(
                "invalid fuel max for ship ID {}: {:?}",
                ship.mst_id, mst
            ))
        })?;
        let max_ammo = mst.api_bull_max.ok_or_else(|| {
            GameplayError::BadManifest(format!(
                "invalid ammo max for ship ID {}: {:?}",
                ship.mst_id, mst
            ))
        })?;

        if ship.fuel < max_fuel {
            return Err(GameplayError::Insufficient(format!(
                "ship {} is not fully fueled: has {}, needs {}",
                ship.id, ship.fuel, max_fuel
            )));
        }
        if ship.ammo < max_ammo {
            return Err(GameplayError::Insufficient(format!(
                "ship {} is not fully supplied with ammo: has {}, needs {}",
                ship.id, ship.ammo, max_ammo
            )));
        }
    }

    Ok(())
}

fn calculate_expedition_supply_costs(
    codex: &Codex,
    mission_mst: &ApiMstMission,
    fleet_ships: &[ship::Model],
) -> Result<Vec<ExpeditionSupplyCost>, GameplayError> {
    let mut costs = Vec::with_capacity(fleet_ships.len());

    for ship in fleet_ships {
        let mst = codex.find::<ApiMstShip>(&ship.mst_id)?;
        let max_fuel = mst.api_fuel_max.ok_or_else(|| {
            GameplayError::BadManifest(format!(
                "invalid fuel max for ship ID {}: {:?}",
                ship.mst_id, mst
            ))
        })?;
        let max_ammo = mst.api_bull_max.ok_or_else(|| {
            GameplayError::BadManifest(format!(
                "invalid ammo max for ship ID {}: {:?}",
                ship.mst_id, mst
            ))
        })?;

        let fuel = calculate_expedition_supply_cost(max_fuel, mission_mst.api_use_fuel);
        let ammo = calculate_expedition_supply_cost(max_ammo, mission_mst.api_use_bull);

        costs.push(ExpeditionSupplyCost {
            ship_id: ship.id,
            fuel,
            ammo,
        });
    }

    Ok(costs)
}

fn calculate_expedition_supply_cost(max_supply: i64, ratio: f64) -> i64 {
    if ratio <= 0.0 || max_supply <= 0 {
        return 0;
    }

    let cost = (max_supply as f64 * ratio).floor() as i64;
    cost.max(1)
}

fn calculate_forced_return_time(
    now: DateTime<Utc>,
    current_return_time: DateTime<Utc>,
    mission_mst: &ApiMstMission,
) -> DateTime<Utc> {
    let total_duration = Duration::minutes(mission_mst.api_time);
    let remaining = (current_return_time - now).max(Duration::zero());
    let elapsed = (total_duration - remaining).max(Duration::zero());
    let forced_duration = std::cmp::min(remaining, elapsed) / 3;

    now + forced_duration
}

async fn apply_expedition_return_morale<C>(
    c: &C,
    mission_mst: &ApiMstMission,
    ships: &[ship::Model],
) -> Result<Vec<ship::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut updated_ships = Vec::with_capacity(ships.len());
    for ship in ships {
        let loss = expedition_return_morale_loss(mission_mst);
        let mut am = (*ship).into_active_model();
        am.condition = ActiveValue::Set((ship.condition - loss).max(0));
        updated_ships.push(am.update(c).await?);
    }

    Ok(updated_ships)
}

fn expedition_return_morale_loss(mission_mst: &ApiMstMission) -> i64 {
    match mission_mst.api_id {
        33 | 203 => rng::i64_inclusive(1..=5),
        34 | 204 => rng::i64_inclusive(1..=10),
        _ => 3,
    }
}

fn determine_expedition_success_result(
    expedition_condition: &Kc3rdExpeditionCondition,
    launch_snapshot: ExpeditionLaunchSnapshot,
) -> ExpeditionResult {
    let great_success_rate = calculate_great_success_rate(expedition_condition, launch_snapshot);
    let great_success_rate = great_success_rate.clamp(0.0, 100.0);
    let roll = rng::f64_range(0.0, 100.0);

    if roll < great_success_rate {
        ExpeditionResult::GreatSuccess
    } else {
        ExpeditionResult::Success
    }
}

fn calculate_great_success_rate(
    expedition_condition: &Kc3rdExpeditionCondition,
    launch_snapshot: ExpeditionLaunchSnapshot,
) -> f64 {
    match great_success_type(expedition_condition.code.as_str()) {
        GreatSuccessType::Type1 => calculate_type1_great_success_rate(
            launch_snapshot.sparkled_ship_count,
            launch_snapshot.fleet_ship_count,
        ),
        GreatSuccessType::Type2 => {
            let is_over_drum = is_type2_over_drum(expedition_condition, launch_snapshot);
            calculate_type2_great_success_rate(launch_snapshot.sparkled_ship_count, is_over_drum)
        }
        GreatSuccessType::Type3 => calculate_type3_great_success_rate(
            launch_snapshot.flagship_level,
            launch_snapshot.sparkled_ship_count,
        ),
    }
}

fn great_success_type(code: &str) -> GreatSuccessType {
    match code {
        "21" | "24" | "37" | "38" | "40" | "44" | "E2" => GreatSuccessType::Type2,
        "A2" | "A3" | "A4" | "A5" | "A6" | "B3" | "B4" | "B5" | "41" | "43" | "45" | "E1" => {
            GreatSuccessType::Type3
        }
        _ => GreatSuccessType::Type1,
    }
}

fn sparkled_ship_count(fleet_ships: &[ship::Model]) -> i64 {
    fleet_ships.iter().filter(|ship| ship.condition >= 50).count() as i64
}

fn calculate_type1_great_success_rate(sparkled_ship_count: i64, fleet_ship_count: i64) -> f64 {
    if fleet_ship_count == 0 || sparkled_ship_count != fleet_ship_count {
        return 0.0;
    }

    (15 * sparkled_ship_count + 21) as f64
}

fn calculate_type2_great_success_rate(sparkled_ship_count: i64, is_over_drum: bool) -> f64 {
    let modifier = if is_over_drum {
        41
    } else {
        6
    };
    (15 * sparkled_ship_count + modifier) as f64
}

fn calculate_type3_great_success_rate(flagship_level: i64, sparkled_ship_count: i64) -> f64 {
    10.0 * (flagship_level.max(0) as f64).sqrt() + 0.1 * (sparkled_ship_count.pow(3) as f64)
}

fn is_type2_over_drum(
    expedition_condition: &Kc3rdExpeditionCondition,
    launch_snapshot: ExpeditionLaunchSnapshot,
) -> bool {
    let Some((required_carriers, required_drums)) =
        type2_over_drum_requirements(expedition_condition.code.as_str())
    else {
        return false;
    };

    launch_snapshot.drum_ship_count >= required_carriers
        && launch_snapshot.total_drums >= required_drums
}

fn type2_over_drum_requirements(code: &str) -> Option<(i64, i64)> {
    match code {
        "21" => Some((3, 4)),
        "24" => Some((1, 2)),
        "37" | "E2" => Some((3, 5)),
        "38" => Some((4, 9)),
        "40" => Some((1, 5)),
        "44" => Some((3, 7)),
        _ => None,
    }
}

fn calculate_expedition_admiral_exp(
    expedition_condition: &Kc3rdExpeditionCondition,
    result: ExpeditionResult,
) -> i64 {
    match result {
        ExpeditionResult::GreatSuccess => expedition_condition.admiral_exp * 2,
        ExpeditionResult::Success => expedition_condition.admiral_exp,
        ExpeditionResult::Failure => 0,
    }
}

fn calculate_failed_expedition_admiral_exp(expedition_condition: &Kc3rdExpeditionCondition) -> i64 {
    ((expedition_condition.admiral_exp as f64) * 0.3).floor() as i64
}

fn calculate_expedition_resource_reward(
    expedition_condition: &Kc3rdExpeditionCondition,
    result: ExpeditionResult,
) -> [i64; 4] {
    match result {
        ExpeditionResult::GreatSuccess => expedition_condition
            .resource_reward
            .map(|amount| ((amount as f64) * 1.5).floor() as i64),
        ExpeditionResult::Success => expedition_condition.resource_reward,
        ExpeditionResult::Failure => [0; 4],
    }
}

async fn apply_expedition_supply_costs<C>(
    c: &C,
    fleet_ships: &[ship::Model],
    supply_costs: &[ExpeditionSupplyCost],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    for ship in fleet_ships {
        let cost = supply_costs.iter().find(|cost| cost.ship_id == ship.id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("supply cost for ship {}", ship.id))
        })?;

        if ship.fuel < cost.fuel {
            return Err(GameplayError::Insufficient(format!(
                "ship {} fuel: has {}, needs {}",
                ship.id, ship.fuel, cost.fuel
            )));
        }
        if ship.ammo < cost.ammo {
            return Err(GameplayError::Insufficient(format!(
                "ship {} ammo: has {}, needs {}",
                ship.id, ship.ammo, cost.ammo
            )));
        }
    }

    for ship in fleet_ships {
        let cost = supply_costs.iter().find(|cost| cost.ship_id == ship.id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("supply cost for ship {}", ship.id))
        })?;

        let mut am = (*ship).into_active_model();
        am.fuel = ActiveValue::Set(ship.fuel - cost.fuel);
        am.ammo = ActiveValue::Set(ship.ammo - cost.ammo);
        am.update(c).await?;
    }

    Ok(())
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
    am.launch_fleet_ship_count = ActiveValue::Set(0);
    am.launch_sparkled_ship_count = ActiveValue::Set(0);
    am.launch_flagship_level = ActiveValue::Set(0);
    am.launch_drum_ship_count = ActiveValue::Set(0);
    am.launch_total_drums = ActiveValue::Set(0);

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
    result: ExpeditionResult,
) -> Result<(Vec<i64>, Vec<[i64; 2]>), GameplayError>
where
    C: ConnectionTrait,
{
    let mut gains = Vec::with_capacity(ships.len());
    let mut after = Vec::with_capacity(ships.len());

    let base_gain = match result {
        ExpeditionResult::GreatSuccess => fleet_exp.max(0) * 2,
        ExpeditionResult::Success | ExpeditionResult::Failure => fleet_exp.max(0),
    };

    for (idx, model) in ships.iter().enumerate() {
        let gain = if !model.married && model.level >= 99 {
            0
        } else if idx == 0 {
            ((base_gain as f64) * 1.5).floor() as i64
        } else {
            base_gain
        };
        gains.push(gain);

        let mut updated = *model;
        updated.exp_now += gain;

        let (level, next_exp) = level::exp_to_ship_level(updated.exp_now);
        let level = level.min(level::ship_level_cap(updated.married));
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
    resource_reward: [i64; 4],
    result: ExpeditionResult,
) -> Result<[Option<ExpeditionItemReward>; 2], GameplayError>
where
    C: ConnectionTrait,
{
    let mats = [
        (MaterialCategory::Fuel, resource_reward[0]),
        (MaterialCategory::Ammo, resource_reward[1]),
        (MaterialCategory::Steel, resource_reward[2]),
        (MaterialCategory::Bauxite, resource_reward[3]),
    ]
    .into_iter()
    .filter(|(_, amount)| *amount > 0)
    .collect::<Vec<_>>();

    if !mats.is_empty() {
        add_material_impl(c, codex, profile_id, &mats).await?;
    }

    let planned_rewards = resolve_expedition_item_rewards(expedition_condition, result);
    let mut items: [Option<ExpeditionItemReward>; 2] = [None, None];
    for (idx, reward) in planned_rewards.into_iter().enumerate() {
        let Some(reward) = reward else {
            continue;
        };

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
        items[idx] = Some(ExpeditionItemReward {
            item_id: reward.item_id,
            count: reward.count,
            name,
        });
    }

    Ok(items)
}

fn resolve_expedition_item_rewards(
    expedition_condition: &Kc3rdExpeditionCondition,
    result: ExpeditionResult,
) -> [Option<ExpeditionItemReward>; 2] {
    let mut rewards: [Option<ExpeditionItemReward>; 2] = [None, None];

    for (idx, reward) in expedition_condition.item_rewards.iter().take(2).enumerate() {
        let count = match idx {
            0 => draw_left_item_reward_count(reward.count, result),
            1 => draw_right_item_reward_count(reward.count, result),
            _ => 0,
        };

        if count > 0 {
            rewards[idx] = Some(ExpeditionItemReward {
                item_id: reward.item_id,
                count,
                name: String::new(),
            });
        }
    }

    rewards
}

fn draw_left_item_reward_count(max_count: i64, result: ExpeditionResult) -> i64 {
    if max_count <= 0 || result == ExpeditionResult::Failure {
        return 0;
    }

    let roll = rng::i64_inclusive(0..=max_count);
    resolve_left_item_reward_count(max_count, result, roll)
}

fn draw_right_item_reward_count(max_count: i64, result: ExpeditionResult) -> i64 {
    if max_count <= 0 || result != ExpeditionResult::GreatSuccess {
        return 0;
    }

    let roll = rng::i64_inclusive(1..=max_count);
    resolve_right_item_reward_count(max_count, result, roll)
}

fn resolve_left_item_reward_count(max_count: i64, result: ExpeditionResult, roll: i64) -> i64 {
    if max_count <= 0 || result == ExpeditionResult::Failure {
        return 0;
    }

    roll.clamp(0, max_count)
}

fn resolve_right_item_reward_count(max_count: i64, result: ExpeditionResult, roll: i64) -> i64 {
    if max_count <= 0 || result != ExpeditionResult::GreatSuccess {
        return 0;
    }

    roll.clamp(1, max_count)
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

fn idle_fleet_err(fleet_id: i64, context: &str) -> GameplayError {
    GameplayError::WrongType(format!("fleet {fleet_id} is idle, {context}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type1_great_success_rate_is_guaranteed_with_six_sparkled() {
        assert_eq!(calculate_type1_great_success_rate(6, 6), 111.0);
        assert_eq!(calculate_type1_great_success_rate(5, 6), 0.0);
    }

    #[test]
    fn type2_great_success_rate_uses_overdrum_modifier() {
        assert_eq!(calculate_type2_great_success_rate(4, false), 66.0);
        assert_eq!(calculate_type2_great_success_rate(4, true), 101.0);
    }

    #[test]
    fn type3_great_success_rate_matches_formula() {
        assert_eq!(calculate_type3_great_success_rate(33, 5).round(), 70.0);
    }

    #[test]
    fn great_success_type_mapping_covers_special_codes() {
        assert_eq!(great_success_type("37"), GreatSuccessType::Type2);
        assert_eq!(great_success_type("A2"), GreatSuccessType::Type3);
        assert_eq!(great_success_type("8"), GreatSuccessType::Type1);
    }

    #[test]
    fn great_success_reward_multipliers_are_applied() {
        let expedition_condition = Kc3rdExpeditionCondition {
            api_id: 8,
            code: "8".to_string(),
            area: 1,
            name: emukc_model::thirdparty::Kc3rdExpeditionName {
                ja: "test".to_string(),
                ko: "test".to_string(),
                en: "test".to_string(),
                zh_cn: "test".to_string(),
                zh_tw: "test".to_string(),
            },
            time_minutes: 180,
            resource_reward: [50, 100, 50, 50],
            item_rewards: vec![],
            admiral_exp: 120,
            fleet_exp: 140,
            requirements: emukc_model::thirdparty::Kc3rdExpeditionRequirements {
                ship_count: 6,
                flagship_level: Some(6),
                fleet_level: None,
                flagship_type: None,
                composition: vec![],
                total_firepower: None,
                total_asw: None,
                total_los: None,
                drum_requirements: None,
            },
        };

        assert_eq!(
            calculate_expedition_resource_reward(
                &expedition_condition,
                ExpeditionResult::GreatSuccess
            ),
            [75, 150, 75, 75]
        );
        assert_eq!(
            calculate_expedition_admiral_exp(&expedition_condition, ExpeditionResult::GreatSuccess),
            240
        );
    }

    #[test]
    fn left_item_reward_count_uses_zero_to_max_distribution() {
        assert_eq!(resolve_left_item_reward_count(2, ExpeditionResult::Success, 0), 0);
        assert_eq!(resolve_left_item_reward_count(2, ExpeditionResult::Success, 1), 1);
        assert_eq!(resolve_left_item_reward_count(2, ExpeditionResult::Success, 2), 2);
        assert_eq!(resolve_left_item_reward_count(2, ExpeditionResult::GreatSuccess, 2), 2);
        assert_eq!(resolve_left_item_reward_count(2, ExpeditionResult::Failure, 2), 0);
    }

    #[test]
    fn right_item_reward_count_is_great_success_only() {
        assert_eq!(resolve_right_item_reward_count(3, ExpeditionResult::Success, 3), 0);
        assert_eq!(resolve_right_item_reward_count(3, ExpeditionResult::Failure, 3), 0);
        assert_eq!(resolve_right_item_reward_count(3, ExpeditionResult::GreatSuccess, 1), 1);
        assert_eq!(resolve_right_item_reward_count(3, ExpeditionResult::GreatSuccess, 3), 3);
    }
}
