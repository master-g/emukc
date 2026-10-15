use emukc_crypto::SimpleHash;
use emukc_db::{
    entity::profile::{self, kdock, ndock},
    sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::{KcApiUserBasic, KcUseItemType},
};
use emukc_time::chrono::{DateTime, Datelike, FixedOffset, Utc};

use crate::{err::GameplayError, gameplay::Ctx};

use super::{
    fleet::get_fleets_impl, furniture::get_furniture_config_impl, kdock::get_kdocks_impl,
    use_item::find_use_item_impl,
};

impl Ctx {
    /// Get user basics.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_user_basic(
        &self,
        profile_id: i64,
    ) -> Result<(profile::Model, KcApiUserBasic), GameplayError> {
        let db = self.db.as_ref();

        let (model, basic) = get_user_basic_impl(db, profile_id).await?;

        Ok((model, basic))
    }

    /// Update user nickname.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `nickname`: The new nickname.
    pub async fn update_user_nickname(
        &self,
        profile_id: i64,
        nickname: &str,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_user_nickname_impl(&tx, profile_id, nickname).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update user comment.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `comment`: The new comment.
    pub async fn update_user_comment(
        &self,
        profile_id: i64,
        comment: &str,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_user_comment_impl(&tx, profile_id, comment).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update user firstflag.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `firstflag`: The new firstflag.
    pub async fn update_user_first_flag(
        &self,
        profile_id: i64,
        firstflag: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        update_user_first_flag_impl(&tx, profile_id, firstflag).await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update user tutorial progress.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `progress`: The new tutorial progress.
    pub async fn update_tutorial_progress(
        &self,
        profile_id: i64,
        progress: i64,
    ) -> Result<(), GameplayError> {
        let db = self.db.as_ref();
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
    let api_count_ndock = ndock::Entity::find()
        .filter(ndock::Column::ProfileId.eq(profile_id))
        .filter(ndock::Column::Status.ne(ndock::Status::Locked))
        .count(c)
        .await? as i64;
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
        api_war_result: war_result(&record, Utc::now()),
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

pub(crate) async fn inc_parallel_quest_max_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let max_quests = profile.max_quests;

    if max_quests >= codex.manifest.api_mst_const.api_parallel_quest_max.api_int_value {
        error!("Capacity exceeded: max_quests={}", max_quests);
        return Ok(());
    }

    let mut am: profile::ActiveModel = profile.into();

    am.id = ActiveValue::Unchanged(profile_id);
    am.max_quests = ActiveValue::Set(max_quests + 1);

    am.update(c).await?;

    Ok(())
}

pub(crate) async fn expand_ship_slotitem_capacity_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let new_ship_capacity = profile.max_ship_capacity + 10;
    let new_slotitem_capacity = profile.max_equipment_capacity + 40;

    let ship_limit = codex.manifest.api_mst_const.api_boko_max_ships.api_int_value;
    if new_ship_capacity > ship_limit {
        error!("Capacity exceeded: new_ship_capacity={}", new_ship_capacity);
        return Ok(());
    }

    let mut am = profile.into_active_model();
    am.max_ship_capacity = ActiveValue::Set(new_ship_capacity);
    am.max_equipment_capacity = ActiveValue::Set(new_slotitem_capacity);

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

/// 戦果 for every 10000 admiral experience: 「獲得戦果 = 提督経験値 × 7 ÷ 10,000」
/// (anastasiatetris.com kancolle-teitoku-exp-senka; kankore-sheep.blog.jp 82941742).
const WAR_RESULT_PER_10000_EXP: i64 = 7;

/// The JST month a moment falls in, as year × 100 + month.
fn jst_month(now: DateTime<Utc>) -> i64 {
    let jst = now.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
    i64::from(jst.year()) * 100 + i64::from(jst.month())
}

/// Start this month's 戦果 if the profile's is another month's: experience from here on
/// counts and the bonus starts at nothing. Call it before experience or bonus is added, so
/// that `experience` still holds what came before.
pub(crate) fn roll_war_result(am: &mut profile::ActiveModel, now: DateTime<Utc>) {
    let month = jst_month(now);
    if *am.war_result_month.as_ref() == month {
        return;
    }
    am.war_result_month = ActiveValue::Set(month);
    am.war_result_base_exp = ActiveValue::Set(*am.experience.as_ref());
    am.war_result = ActiveValue::Set(0);
}

/// This month's 戦果: 7 for every 10000 experience since the month started, and the bonus
/// (quests, 1-6's goal). Nothing while no experience or bonus has come this month.
// ponytail: the month-end limits (21:59 for 戦果) are #147's.
pub(crate) fn war_result(record: &profile::Model, now: DateTime<Utc>) -> i64 {
    if record.war_result_month != jst_month(now) {
        return 0;
    }
    (record.experience - record.war_result_base_exp) * WAR_RESULT_PER_10000_EXP / 10_000
        + record.war_result
}

pub(crate) async fn add_war_result_impl<C>(
    c: &C,
    profile_id: i64,
    amount: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let mut am = profile.into_active_model();
    roll_war_result(&mut am, Utc::now());
    am.war_result = ActiveValue::Set(am.war_result.as_ref() + amount);
    am.update(c).await?;
    Ok(())
}

pub(crate) async fn unlock_expedition_supply_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let mut am = profile.into_active_model();
    am.extra_supply_expedition = ActiveValue::Set(true);
    am.update(c).await?;
    Ok(())
}

pub(crate) async fn unlock_large_construction_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let mut am = profile.into_active_model();
    am.large_dock_unlocked = ActiveValue::Set(true);
    am.update(c).await?;
    Ok(())
}

#[cfg(test)]
mod war_result_tests {
    use emukc_db::{
        entity::profile,
        sea_orm::{ActiveValue, IntoActiveModel, TryIntoModel},
    };
    use emukc_time::chrono::{DateTime, TimeZone, Utc};

    use super::{roll_war_result, war_result};

    /// 2026-10-31 15:30 UTC is 2026-11-01 00:30 JST.
    fn utc(month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, month, day, hour, 30, 0).unwrap()
    }

    fn record(experience: i64) -> profile::Model {
        let mut am = profile::default_active_model(1, "admiral");
        am.id = ActiveValue::Set(1);
        am.experience = ActiveValue::Set(experience);
        am.try_into_model().unwrap()
    }

    /// Add experience and bonus the way the callers do: roll first, then add.
    fn gain(record: profile::Model, now: DateTime<Utc>, exp: i64, bonus: i64) -> profile::Model {
        let mut am = record.into_active_model();
        roll_war_result(&mut am, now);
        am.experience = ActiveValue::Set(am.experience.as_ref() + exp);
        am.war_result = ActiveValue::Set(am.war_result.as_ref() + bonus);
        am.try_into_model().unwrap()
    }

    #[test]
    fn a_month_counts_seven_in_ten_thousand_of_experience_and_its_bonus() {
        // Experience from before the month started does not count.
        let r = gain(record(500_000), utc(10, 2, 0), 10_000, 0);
        assert_eq!(war_result(&r, utc(10, 2, 1)), 7);
        let r = gain(r, utc(10, 3, 0), 4_285, 75);
        // 14285 × 7 / 10000 = 9.9995, down to 9; and the bonus.
        assert_eq!(war_result(&r, utc(10, 3, 1)), 9 + 75);
    }

    #[test]
    fn a_new_month_in_japan_starts_from_nothing() {
        let r = gain(record(0), utc(10, 2, 0), 20_000, 75);
        assert_eq!(war_result(&r, utc(10, 31, 14)), 14 + 75);
        // Read in November (JST) before anything is gained: nothing yet.
        assert_eq!(war_result(&r, utc(10, 31, 15)), 0);
        // Gained in November: only November's.
        let r = gain(r, utc(10, 31, 16), 10_000, 0);
        assert_eq!(war_result(&r, utc(10, 31, 17)), 7);
    }

    #[test]
    fn a_profile_that_never_started_a_month_has_none() {
        assert_eq!(war_result(&record(1_000_000), utc(10, 2, 0)), 0);
    }
}
