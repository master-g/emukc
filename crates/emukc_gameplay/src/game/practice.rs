use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use async_trait::async_trait;
use emukc_crypto::rng;
use emukc_db::{
    entity::profile::{
        self,
        practice::{self, config::RivalType, rival_ship},
        ship,
    },
    sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::{KcSortieResultRank, UserHQRank, level},
    profile::practice::{PracticeConfig, Rival, RivalDetail, RivalFlag, RivalShip, RivalStatus},
    thirdparty::QuestActionEvent,
};
use emukc_time::{
    KcTime,
    chrono::{DateTime, Duration, Utc},
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
    basic::find_profile,
    battle::practice::{
        PracticeBattleInput, PracticeBattleResponse, PracticeBattleResultResponse,
        PracticeBattleResultSnapshot, PracticeBattleShipInput, PracticeNightBattleResponse,
        build_result_response, calculate_admiral_exp, calculate_ship_exp,
        clear_pending_practice_battle, pending_practice_battle, run_day_battle, run_night_battle,
    },
    fleet::get_fleet_ships_impl,
    quest::update::update_quest_progress_for_action,
    ship::update_ship_impl,
    slot_item::find_slot_items_by_id_impl,
};

static PENDING_PRACTICE_RESULTS: LazyLock<Mutex<HashMap<i64, PracticeBattleResultSnapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

    /// Get practice rival details.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `rival_id`: The rival ID.
    async fn get_practice_rival_details(
        &self,
        profile_id: i64,
        rival_id: i64,
    ) -> Result<Rival, GameplayError>;

    /// Start a practice day battle.
    async fn practice_battle(
        &self,
        profile_id: i64,
        deck_id: i64,
        formation_id: i64,
        enemy_id: i64,
    ) -> Result<PracticeBattleResponse, GameplayError>;

    /// Get the latest practice battle result.
    async fn practice_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<PracticeBattleResultResponse, GameplayError>;

    async fn practice_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<PracticeNightBattleResponse, GameplayError>;
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

    async fn get_practice_rival_details(
        &self,
        profile_id: i64,
        rival_id: i64,
    ) -> Result<Rival, GameplayError> {
        let db = self.db();

        let rival = get_practice_rival_details_impl(db, profile_id, rival_id).await?;

        Ok(rival)
    }

    async fn practice_battle(
        &self,
        profile_id: i64,
        deck_id: i64,
        formation_id: i64,
        enemy_id: i64,
    ) -> Result<PracticeBattleResponse, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let tx = db.begin().await?;

        let rival = get_practice_rival_details_impl(&tx, profile_id, enemy_id).await?;
        let profile = find_profile(&tx, profile_id).await?;
        let friend_ships = get_fleet_ships_impl(&tx, profile_id, deck_id).await?;
        if friend_ships.is_empty() {
            return Err(GameplayError::WrongType(format!(
                "fleet {deck_id} has no ships for practice battle",
            )));
        }

        let friend_ships = build_practice_friend_ships(&tx, &friend_ships).await?;
        let enemy_ships = build_practice_enemy_ships(codex, &rival)?;
        let input = PracticeBattleInput {
            profile_id,
            deck_id,
            formation_id,
            enemy_id,
            friend_ships,
            enemy_ships,
            rival,
            member_lv: profile.hq_level,
            member_exp: profile.experience,
        };

        let (response, snapshot) = run_day_battle(codex, input)?;
        PENDING_PRACTICE_RESULTS.lock().unwrap().insert(profile_id, snapshot);

        tx.commit().await?;

        Ok(response)
    }

    async fn practice_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<PracticeBattleResultResponse, GameplayError> {
        let db = self.db();
        let tx = db.begin().await?;

        let snapshot =
            PENDING_PRACTICE_RESULTS.lock().unwrap().remove(&profile_id).ok_or_else(|| {
                GameplayError::EntryNotFound(format!(
                    "practice battle result not found for profile {profile_id}",
                ))
            })?;

        let snapshot =
            update_practice_result_stats(&tx, self.codex(), profile_id, snapshot).await?;
        update_rival_status(&tx, profile_id, snapshot.enemy_id, &snapshot.win_rank).await?;
        let quest_event = build_practice_quest_event(&snapshot)?;
        update_quest_progress_for_action(&tx, self.codex(), profile_id, &quest_event).await?;
        clear_pending_practice_battle(profile_id);

        tx.commit().await?;

        Ok(build_result_response(snapshot))
    }

    async fn practice_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<PracticeNightBattleResponse, GameplayError> {
        let codex = self.codex();
        let (response, snapshot) = run_night_battle(codex, profile_id).ok_or_else(|| {
            GameplayError::WrongType("night practice battle is not available".to_string())
        })?;

        let ct_flagship = pending_practice_battle(profile_id)
            .and_then(|s| s.friendly.first().map(|f| f.ship.api_ship_id))
            .and_then(|sid| codex.manifest.find_ship(sid))
            .is_some_and(|m| m.api_stype == 21);

        if let Some(existing) = PENDING_PRACTICE_RESULTS.lock().unwrap().get_mut(&profile_id) {
            let base_exp = existing.get_base_exp;
            let friend_ships = pending_practice_battle(profile_id)
                .map(|session| session.friendly)
                .unwrap_or_default();
            existing.win_rank = snapshot.win_rank;
            existing.mvp = snapshot.mvp;
            existing.get_exp = calculate_admiral_exp(base_exp, &existing.win_rank);
            let (ship_exp, ship_lvup) = calculate_ship_exp(
                &friend_ships,
                base_exp,
                existing.mvp,
                ct_flagship,
                codex.game_cfg.exp.ct_exp_boost,
                codex.game_cfg.exp.practice_exp_boost,
            );
            existing.get_ship_exp = ship_exp;
            existing.get_exp_lvup = ship_lvup;
        }

        Ok(response)
    }
}

fn build_practice_quest_event(
    snapshot: &PracticeBattleResultSnapshot,
) -> Result<QuestActionEvent, GameplayError> {
    Ok(QuestActionEvent::ExerciseBattleCompleted {
        fleet_id: snapshot.deck_id,
        win_rank: parse_practice_result_rank(&snapshot.win_rank)?,
        fleet_ships: snapshot.friendly_fleet_snapshot.clone(),
    })
}

fn parse_practice_result_rank(win_rank: &str) -> Result<KcSortieResultRank, GameplayError> {
    match win_rank {
        "S" => Ok(KcSortieResultRank::S),
        "A" => Ok(KcSortieResultRank::A),
        "B" => Ok(KcSortieResultRank::B),
        "C" => Ok(KcSortieResultRank::C),
        "D" => Ok(KcSortieResultRank::D),
        "E" => Ok(KcSortieResultRank::E),
        _ => {
            Err(GameplayError::WrongType(format!("unexpected practice result rank `{win_rank}`",)))
        }
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
                "Practice config not found for profile {profile_id}",
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

        debug!("load practice rivals");

        for rival in rivals.into_iter() {
            // find rival details and all ships
            let details = profile::practice::detail::Entity::find_by_id(rival.id)
                .one(c)
                .await?
                .ok_or_else(|| {
                    GameplayError::EntryNotFound(format!(
                        "Practice rival details not found for profile {profile_id}",
                    ))
                })?;

            let ships = profile::practice::rival_ship::Entity::find()
                .filter(profile::practice::rival_ship::Column::ProfileId.eq(profile_id))
                .filter(profile::practice::rival_ship::Column::RivalId.eq(rival.id))
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

pub(crate) async fn get_practice_rival_details_impl<C>(
    c: &C,
    profile_id: i64,
    rival_id: i64,
) -> Result<Rival, GameplayError>
where
    C: ConnectionTrait,
{
    let rival =
        profile::practice::rival::Entity::find_by_id(rival_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "Practice rival not found for profile {profile_id}",
            ))
        })?;

    let details =
        profile::practice::detail::Entity::find_by_id(rival_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "Practice rival details not found for profile {profile_id}",
            ))
        })?;

    let ships = profile::practice::rival_ship::Entity::find()
        .filter(profile::practice::rival_ship::Column::ProfileId.eq(profile_id))
        .filter(profile::practice::rival_ship::Column::RivalId.eq(rival_id))
        .all(c)
        .await?;

    let rival = Rival {
        id: rival.id,
        index: rival.index,
        name: rival.name,
        comment: rival.comment,
        level: rival.level,
        rank: rival.rank.into(),
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
            ships: ships.into_iter().map(std::convert::Into::into).collect(),
        },
    };

    Ok(rival)
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

    let last_ship = rival_ship::Entity::find().order_by_desc(rival_ship::Column::Id).one(c).await?;
    let current_ship_id = last_ship.map(|s| s.id).unwrap_or(0) + 1;

    let rival_uid_starts_from = current_uid + 10000;
    let rival_ship_id_starts_from = current_ship_id + 10000;

    let rivals: Vec<Rival> = {
        (1..6)
            .map(|i| {
                let name = format!("Practice Rival {i}");
                let comment = format!("I am your {i}th rival");
                let rank = rng::i64_inclusive(1..=10);
                let flag = rng::i64_inclusive(1..=3);
                let ship_mst = rng::choose(&api_mst_ship).unwrap();

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
                        deck_name: format!("Deck {i}"),
                        ships: vec![RivalShip {
                            id: rival_ship_id_starts_from + i,
                            mst_id: ship_mst.api_id,
                            level: 180,
                            star: 10,
                        }],
                    },
                }
            })
            .collect()
    };

    // remove old records

    profile::practice::rival_ship::Entity::delete_many()
        .filter(profile::practice::rival_ship::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    profile::practice::detail::Entity::delete_many()
        .filter(profile::practice::detail::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    profile::practice::rival::Entity::delete_many()
        .filter(profile::practice::rival::Column::ProfileId.eq(profile_id))
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
                let am = profile::practice::rival_ship::ActiveModel {
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

async fn build_practice_friend_ships<C>(
    c: &C,
    friend_ships: &[emukc_db::entity::profile::ship::Model],
) -> Result<Vec<PracticeBattleShipInput>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut result = Vec::with_capacity(friend_ships.len());
    for ship in friend_ships {
        let slot_ids =
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
                .into_iter()
                .filter(|slot_id| *slot_id > 0)
                .collect::<Vec<_>>();
        let slot_items = find_slot_items_by_id_impl(c, &slot_ids).await?;
        let slot_items = slot_items.into_iter().map(std::convert::Into::into).collect();

        result.push(PracticeBattleShipInput {
            ship: (*ship).into(),
            slot_items,
            effect_list: vec![],
            married: ship.married,
        });
    }

    Ok(result)
}

fn build_practice_enemy_ships(
    codex: &Codex,
    rival: &Rival,
) -> Result<Vec<PracticeBattleShipInput>, GameplayError> {
    rival
        .details
        .ships
        .iter()
        .map(|ship| {
            let (mut api_ship, slot_items) =
                codex.new_ship(ship.mst_id).ok_or(GameplayError::ManifestNotFound(ship.mst_id))?;
            let exp_now = level::ship_level_required_exp(ship.level);
            let (_, next_exp) = level::exp_to_ship_level(exp_now);
            api_ship.api_lv = ship.level;
            api_ship.api_exp = [exp_now, next_exp, 0];
            codex.cal_ship_status(&mut api_ship, &slot_items, false)?;

            Ok(PracticeBattleShipInput {
                ship: api_ship,
                slot_items,
                effect_list: vec![0],
                married: false,
            })
        })
        .collect()
}

async fn update_practice_result_stats<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    mut snapshot: PracticeBattleResultSnapshot,
) -> Result<PracticeBattleResultSnapshot, GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let mut am = profile.into_active_model();
    let current_exp = am.experience.take().unwrap_or_default();
    let new_exp = current_exp + snapshot.get_exp;
    let (hq_level, _) = level::exp_to_hq_level(new_exp);
    am.practice_battles = ActiveValue::Set(am.practice_battles.take().unwrap_or_default() + 1);
    if matches!(snapshot.win_rank.as_str(), "S" | "A" | "B") {
        am.practice_battle_wins =
            ActiveValue::Set(am.practice_battle_wins.take().unwrap_or_default() + 1);
    }
    am.experience = ActiveValue::Set(new_exp);
    am.hq_level = ActiveValue::Set(hq_level);
    let updated_profile = am.update(c).await?;
    let pending = pending_practice_battle(profile_id);

    for (idx, ship_id) in snapshot.friendly_ship_ids.iter().copied().enumerate() {
        let gain = snapshot.get_ship_exp.get(idx + 1).copied().unwrap_or(-1);
        let ship_model = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("ship with id {ship_id} not found"))
        })?;
        let mst = codex.find::<emukc_model::prelude::ApiMstShip>(&ship_model.mst_id)?;
        let mut api_ship: emukc_model::kc2::KcApiShip = ship_model.into();
        let new_ship_exp = ship_model.exp_now + gain.max(0);
        let (mut ship_level, next_exp) = level::exp_to_ship_level(new_ship_exp);
        ship_level = ship_level.min(level::ship_level_cap(ship_model.married));
        let current_level_exp = level::ship_level_required_exp(ship_level);
        let progress = if next_exp > current_level_exp {
            ((new_ship_exp - current_level_exp) * 100 / (next_exp - current_level_exp)).clamp(0, 99)
        } else {
            0
        };
        api_ship.api_lv = ship_level;
        api_ship.api_exp = [new_ship_exp, next_exp, progress];
        api_ship.api_fuel =
            (ship_model.fuel - practice_fuel_cost(mst.api_fuel_max.unwrap_or(0))).max(0);
        api_ship.api_bull = (ship_model.ammo
            - practice_ammo_cost(mst.api_bull_max.unwrap_or(0), snapshot.did_night_battle))
        .max(0);
        if let Some(session) = &pending
            && let Some(runtime_ship) = session.friendly.get(idx)
        {
            api_ship.api_onslot = runtime_ship.ship.api_onslot;
        }
        update_ship_impl(c, codex, &api_ship).await?;
    }

    snapshot.member_lv = updated_profile.hq_level;
    snapshot.member_exp = updated_profile.experience;
    Ok(snapshot)
}

fn practice_fuel_cost(max_fuel: i64) -> i64 {
    if max_fuel <= 0 {
        return 0;
    }
    (max_fuel / 5).max(1)
}

fn practice_ammo_cost(max_ammo: i64, did_night_battle: bool) -> i64 {
    if max_ammo <= 0 {
        return 0;
    }
    let base = (max_ammo / 5).max(1);
    if did_night_battle {
        (base * 3 + 1) / 2
    } else {
        base
    }
}

async fn update_rival_status<C>(
    c: &C,
    profile_id: i64,
    enemy_id: i64,
    win_rank: &str,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let rival = practice::rival::Entity::find_by_id(enemy_id)
        .filter(practice::rival::Column::ProfileId.eq(profile_id))
        .one(c)
        .await?
        .ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "practice rival {enemy_id} not found for profile {profile_id}",
            ))
        })?;

    let status = match win_rank {
        "S" => practice::rival::Status::VictoryRankS,
        "A" => practice::rival::Status::VictoryRankA,
        "B" => practice::rival::Status::VictoryRankB,
        "C" => practice::rival::Status::LostRankC,
        "D" => practice::rival::Status::LostRankD,
        _ => practice::rival::Status::LostRankE,
    };

    let mut am = rival.into_active_model();
    am.status = ActiveValue::Set(status);
    am.update(c).await?;
    Ok(())
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
    profile::practice::rival_ship::Entity::delete_many()
        .filter(profile::practice::rival_ship::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    Ok(())
}
