use emukc_crypto::rng::{self, GameRng};
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
};
use emukc_time::{
    KcTime,
    chrono::{DateTime, Duration, Utc},
};

use crate::{err::GameplayError, gameplay::Ctx};

use super::{
    basic::find_profile,
    battle::practice::{
        PracticeBattleInput, PracticeBattleResultResponse, PracticeBattleResultSnapshot,
        PracticeBattleShipInput, build_result_response, calculate_ship_exp, run_day_battle,
        run_night_battle,
    },
    battle::response::{DayBattleResponse, NightBattleResponse},
    battle::rng::ProductionRng,
    fleet::get_fleet_ships_impl,
    quest::observe::{GameplayOutcome, observe},
    ship::{
        exp::{calculate_admiral_exp, settle_ship_exp},
        update_ship_impl,
    },
    slot_item::find_slot_items_by_id_impl,
    sortie_store::PracticeStore,
};

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

impl Ctx {
    /// Choose which group the next practice rivals are drawn from.
    ///
    /// The list in hand stays; the choice applies when it is next drawn, at
    /// 03:00 or 15:00 JST.
    pub async fn change_practice_matching_kind(
        &self,
        profile_id: i64,
        kind: i64,
    ) -> Result<(), GameplayError> {
        let selected = match kind {
            0 => profile::practice::config::RivalType::FirstGroup,
            1 => profile::practice::config::RivalType::SecondGroup,
            2 => profile::practice::config::RivalType::All,
            _ => {
                return Err(GameplayError::WrongType(format!(
                    "practice matching kind {kind} is not one of 0..=2"
                )));
            }
        };

        let db = self.db.as_ref();
        let config = profile::practice::config::Entity::find_by_id(profile_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                GameplayError::EntryNotFound(format!(
                    "Practice config not found for profile {profile_id}",
                ))
            })?;
        let mut am = config.into_active_model();
        am.selected_type = ActiveValue::Set(selected);
        am.update(db).await?;

        Ok(())
    }

    /// Get practice rivals.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    pub async fn get_practice_rivals(
        &self,
        profile_id: i64,
    ) -> Result<PracticeInfo, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let rivals = get_practice_rivals_impl(&tx, codex, profile_id).await?;

        tx.commit().await?;

        Ok(rivals)
    }

    /// Get practice rival details.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `rival_id`: The rival ID.
    pub async fn get_practice_rival_details(
        &self,
        profile_id: i64,
        rival_id: i64,
    ) -> Result<Rival, GameplayError> {
        let db = self.db.as_ref();

        let rival = get_practice_rival_details_impl(db, profile_id, rival_id).await?;

        Ok(rival)
    }

    /// Start a practice day battle.
    pub async fn practice_battle(
        &self,
        profile_id: i64,
        deck_id: i64,
        formation_id: i64,
        enemy_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
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

        let practice_repo = self.practice_store.as_ref();
        let _rng = ProductionRng;
        let mut rng = ProductionRng;
        let (response, snapshot) = run_day_battle(codex, input, practice_repo, &mut rng)?;
        practice_repo.insert_pending_result(profile_id, snapshot);

        tx.commit().await?;

        Ok(response)
    }

    /// Get the latest practice battle result.
    pub async fn practice_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<PracticeBattleResultResponse, GameplayError> {
        let db = self.db.as_ref();
        let tx = db.begin().await?;

        let practice_repo = self.practice_store.as_ref();
        let snapshot = practice_repo.take_pending_result(profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "practice battle result not found for profile {profile_id}",
            ))
        })?;

        let snapshot = update_practice_result_stats(
            &tx,
            self.codex.as_ref(),
            profile_id,
            snapshot,
            practice_repo,
        )
        .await?;
        update_rival_status(&tx, profile_id, snapshot.enemy_id, &snapshot.win_rank.to_string())
            .await?;
        let outcome = build_practice_battle_outcome(&snapshot)?;
        observe(&tx, self.codex.as_ref(), profile_id, &[outcome]).await?;
        practice_repo.clear_pending_battle(profile_id);

        tx.commit().await?;

        Ok(build_result_response(snapshot))
    }

    pub async fn practice_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<NightBattleResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let practice_repo = self.practice_store.as_ref();
        let mut rng = ProductionRng;
        let (response, snapshot) = run_night_battle(codex, profile_id, practice_repo, &mut rng)
            .ok_or_else(|| {
                GameplayError::WrongType("night practice battle is not available".to_string())
            })?;

        let ct_flagship = practice_repo
            .get_pending_battle(profile_id)
            .and_then(|s| s.friendly.first().map(|f| f.ship.api_ship_id))
            .and_then(|sid| codex.manifest.find_ship(sid))
            .is_some_and(|m| m.api_stype == 21);

        if let Some(mut existing) = practice_repo.take_pending_result(profile_id) {
            let base_exp = existing.get_base_exp;
            let friend_ships = practice_repo
                .get_pending_battle(profile_id)
                .map(|session| session.friendly)
                .unwrap_or_default();
            existing.win_rank = snapshot.win_rank;
            existing.mvp = snapshot.mvp;
            existing.get_exp = calculate_admiral_exp(base_exp, &existing.win_rank.to_string());
            let (ship_exp, ship_lvup) = calculate_ship_exp(
                &friend_ships,
                base_exp,
                existing.win_rank,
                existing.mvp,
                ct_flagship,
                codex.game_cfg.exp.ct_exp_boost,
                codex.game_cfg.exp.practice_exp_boost,
            );
            existing.get_ship_exp = ship_exp;
            existing.get_exp_lvup = ship_lvup;
            practice_repo.insert_pending_result(profile_id, existing);
        }

        Ok(response)
    }
}

fn build_practice_battle_outcome(
    snapshot: &PracticeBattleResultSnapshot,
) -> Result<GameplayOutcome, GameplayError> {
    Ok(GameplayOutcome::ExerciseBattleCompleted {
        fleet_id: snapshot.deck_id,
        win_rank: snapshot.win_rank,
        fleet_ships: snapshot.friendly_fleet_snapshot.clone(),
    })
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
                .order_by_asc(profile::practice::rival_ship::Column::Id)
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
        // Fleet order: the flagship has the lowest id.
        .order_by_asc(profile::practice::rival_ship::Column::Id)
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

/// Generate practice rivals for the player's HQ level and practice group,
/// replacing the stored ones.
async fn generate_practice_rivals<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    select_type: RivalType,
) -> Result<Vec<Rival>, GameplayError>
where
    C: ConnectionTrait,
{
    let hq_level = find_profile(c, profile_id).await?.hq_level;

    let last_profile = profile::Entity::find().order_by_desc(profile::Column::Id).one(c).await?;
    let current_uid = last_profile.map(|p| p.id).unwrap_or(0) + 1;

    let last_ship = rival_ship::Entity::find().order_by_desc(rival_ship::Column::Id).one(c).await?;
    let current_ship_id = last_ship.map(|s| s.id).unwrap_or(0) + 1;

    let rival_uid_starts_from = current_uid + 10000;
    let rival_ship_id_starts_from = current_ship_id + 10000;

    let rivals = draw_practice_rivals(
        codex,
        hq_level,
        select_type,
        rival_uid_starts_from,
        rival_ship_id_starts_from,
        &GameRng::seeded(rng::u64(0..u64::MAX)),
    );

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

/// Names and comments for generated practice rivals; display only.
const RIVAL_NAMES: [&str; 12] = [
    "横須賀の提督",
    "呉の古参提督",
    "佐世保の司令官",
    "舞鶴の新米提督",
    "大湊の常連",
    "トラック泊地",
    "ブイン基地",
    "ショートランド泊地",
    "リンガ泊地",
    "パラオ泊地",
    "単冠湾の提督",
    "幌筵の司令官",
];
const RIVAL_COMMENTS: [&str; 12] = [
    "よろしくお願いします",
    "演習お手柔らかに",
    "毎日演習しています",
    "遠征中心でのんびり",
    "イベント準備中",
    "改修資材が足りない",
    "練度上げ中です",
    "資源を貯めています",
    "S勝利目指して",
    "新艦娘を育成中",
    "夜戦が好きです",
    "艦隊これくしょん楽しい",
];

/// Draw five practice rivals for a player at `hq_level` from `group`.
///
/// The first group sits at [P, P+20], the second at [P-10, P+10], clamped to
/// 1-120; "all" picks one of the two per rival. Each rival has 1-6 ships, each
/// at a level in [max(1, H/2), min(H, 99)], in a form it can have reached by
/// then, and no two forms of one ship in a fleet.
fn draw_practice_rivals(
    codex: &Codex,
    hq_level: i64,
    group: RivalType,
    rival_id_base: i64,
    ship_id_base: i64,
    rng: &GameRng,
) -> Vec<Rival> {
    // Ships a player can own, with the level each form is remodelled at.
    let pool: Vec<(i64, i64)> = codex
        .ship_extra
        .iter()
        .map(|(id, extra)| (*id, extra.remodel.as_ref().map_or(1, |r| r.level.max(1))))
        .collect();
    let mut ship_id = ship_id_base;

    (1..=5)
        .map(|index| {
            let first_group = match group {
                RivalType::FirstGroup => true,
                RivalType::SecondGroup => false,
                RivalType::All => rng.i64_inclusive(0..=1) == 0,
            };
            let (lo, hi) = if first_group {
                (hq_level, hq_level + 20)
            } else {
                (hq_level - 10, hq_level + 10)
            };
            let level = rng.i64_inclusive(lo.clamp(1, 120)..=hi.clamp(1, 120));

            // Every form of the ships already in this fleet.
            let mut taken: Vec<i64> = Vec::new();
            let mut ships = Vec::new();
            for _ in 0..rng.i64_inclusive(1..=6) {
                let ship_level = rng.i64_inclusive((level / 2).max(1)..=level.min(99));
                let mut candidates: Vec<i64> = pool
                    .iter()
                    .filter(|(id, min_level)| *min_level <= ship_level && !taken.contains(id))
                    .map(|(id, _)| *id)
                    .collect();
                while !candidates.is_empty() {
                    let mst_id = candidates.swap_remove(rng.usize(0..candidates.len()));
                    let chain =
                        codex.ships_before_and_after(mst_id).unwrap_or_else(|_| vec![mst_id]);
                    if chain.iter().any(|form| taken.contains(form)) {
                        continue;
                    }
                    taken.extend(chain);
                    ship_id += 1;
                    ships.push(RivalShip {
                        id: ship_id,
                        mst_id,
                        level: ship_level,
                        star: 10,
                    });
                    break;
                }
            }

            let flagship_name = ships
                .first()
                .and_then(|ship| codex.manifest.find_ship(ship.mst_id))
                .map_or("", |mst| mst.api_name.as_str());
            let rank = rng.i64_inclusive(1..=10);
            let flag = rng.i64_inclusive(1..=3);

            Rival {
                id: rival_id_base + index,
                index,
                name: String::from(*rng.choose(&RIVAL_NAMES).unwrap()),
                comment: String::from(*rng.choose(&RIVAL_COMMENTS).unwrap()),
                level,
                rank: UserHQRank::n(rank).unwrap_or(UserHQRank::Admiral),
                flag: RivalFlag::n(flag).unwrap_or_default(),
                status: RivalStatus::Untouched,
                medals: 10 + index,
                details: RivalDetail {
                    exp_now: 0,
                    exp_next: 1000,
                    friend: 0,
                    current_ship_count: 299,
                    ship_capacity: 300,
                    current_slot_item_count: 999,
                    slot_item_capacity: 1000,
                    furniture: 123,
                    deck_name: format!("{flagship_name}艦隊"),
                    ships,
                },
            }
        })
        .collect()
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
    practice_repo: &PracticeStore,
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
    if matches!(
        snapshot.win_rank,
        KcSortieResultRank::S | KcSortieResultRank::A | KcSortieResultRank::B
    ) {
        am.practice_battle_wins =
            ActiveValue::Set(am.practice_battle_wins.take().unwrap_or_default() + 1);
    }
    am.experience = ActiveValue::Set(new_exp);
    am.hq_level = ActiveValue::Set(hq_level);
    let updated_profile = am.update(c).await?;

    for (idx, ship_id) in snapshot.friendly_ship_ids.iter().copied().enumerate() {
        let gain = snapshot.get_ship_exp.get(idx + 1).copied().unwrap_or(-1);
        let ship_model = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("ship with id {ship_id} not found"))
        })?;
        let mst = codex.find::<emukc_model::prelude::ApiMstShip>(&ship_model.mst_id)?;
        let mut api_ship: emukc_model::kc2::KcApiShip = ship_model.into();
        let settled = settle_ship_exp(ship_model.exp_now, gain.max(0), ship_model.married);
        api_ship.api_lv = settled.level;
        api_ship.api_exp = [settled.exp_now, settled.exp_next, settled.progress];
        api_ship.api_fuel =
            (ship_model.fuel - practice_fuel_cost(mst.api_fuel_max.unwrap_or(0))).max(0);
        api_ship.api_bull = (ship_model.ammo
            - practice_ammo_cost(mst.api_bull_max.unwrap_or(0), snapshot.did_night_battle))
        .max(0);
        if let Some(session) = practice_repo.get_pending_battle(profile_id).as_ref()
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

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use crate::game::battle::practice::response::calculate_base_exp;

    static CODEX: LazyLock<Codex> =
        LazyLock::new(|| Codex::load_without_cache_source("../../.data/codex").unwrap());

    const GROUPS: [RivalType; 3] = [RivalType::FirstGroup, RivalType::SecondGroup, RivalType::All];

    /// Every batch of rivals drawn for `hq_level` and `group` over a range of seeds.
    fn batches(hq_level: i64, group: RivalType) -> impl Iterator<Item = Vec<Rival>> {
        (0..20).map(move |seed| {
            let rivals =
                draw_practice_rivals(&CODEX, hq_level, group, 0, 0, &GameRng::seeded(seed));
            assert_eq!(rivals.len(), 5);
            rivals
        })
    }

    #[test]
    fn rival_hq_levels_follow_the_group() {
        for hq_level in [1, 60, 120] {
            let first = clamped(hq_level, hq_level + 20);
            let second = clamped(hq_level - 10, hq_level + 10);
            for group in GROUPS {
                for rivals in batches(hq_level, group) {
                    for rival in &rivals {
                        let h = rival.level;
                        let ok = match group {
                            RivalType::FirstGroup => first.contains(&h),
                            RivalType::SecondGroup => second.contains(&h),
                            RivalType::All => first.contains(&h) || second.contains(&h),
                        };
                        assert!(ok, "P={hq_level} {group:?}: H={h}");
                    }
                }
            }
        }
    }

    /// Batches for P = 1, 60 and 120 in every group.
    fn all_batches() -> impl Iterator<Item = Vec<Rival>> {
        [1, 60, 120]
            .into_iter()
            .flat_map(|hq_level| GROUPS.into_iter().flat_map(move |group| batches(hq_level, group)))
    }

    fn clamped(lo: i64, hi: i64) -> std::ops::RangeInclusive<i64> {
        lo.clamp(1, 120)..=hi.clamp(1, 120)
    }

    #[test]
    fn rival_fleets_hold_one_to_six_ships_at_reachable_levels() {
        for hq_level in [1, 60, 120] {
            for group in GROUPS {
                for rivals in batches(hq_level, group) {
                    for rival in &rivals {
                        let h = rival.level;
                        let ships = &rival.details.ships;
                        assert!((1..=6).contains(&ships.len()), "{} ships", ships.len());
                        for ship in ships {
                            assert!(
                                ((h / 2).max(1)..=h.min(99)).contains(&ship.level),
                                "H={h}: ship level {}",
                                ship.level
                            );
                            let extra = CODEX.ship_extra.get(&ship.mst_id).unwrap_or_else(|| {
                                panic!("{} is not a ship a player can own", ship.mst_id)
                            });
                            let min_level = extra.remodel.as_ref().map_or(1, |r| r.level.max(1));
                            assert!(
                                ship.level >= min_level,
                                "{} at level {} is below its remodel level {min_level}",
                                ship.mst_id,
                                ship.level
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_rival_fleet_never_holds_two_forms_of_one_ship() {
        for rivals in all_batches() {
            for rival in &rivals {
                let ships = &rival.details.ships;
                for (i, ship) in ships.iter().enumerate() {
                    let chain = CODEX.ships_before_and_after(ship.mst_id).unwrap();
                    for other in &ships[i + 1..] {
                        assert!(
                            !chain.contains(&other.mst_id),
                            "{} and {} are forms of one ship",
                            ship.mst_id,
                            other.mst_id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rivals_are_named_from_the_lists_and_their_flagship() {
        for rivals in all_batches() {
            for rival in &rivals {
                assert!(RIVAL_NAMES.contains(&rival.name.as_str()), "{}", rival.name);
                assert!(RIVAL_COMMENTS.contains(&rival.comment.as_str()), "{}", rival.comment);
                let flagship = CODEX.manifest.find_ship(rival.details.ships[0].mst_id).unwrap();
                assert_eq!(rival.details.deck_name, format!("{}艦隊", flagship.api_name));
            }
        }
    }

    /// Balance regression: base exp reads the rivals' first two ships (plan
    /// 2026-10-14-007). Before, it was the rival's HQ level × 9 clamped to
    /// 100-1200: 100-270 at P=10, 540-720 (first) and 450-630 (second) at P=60.
    #[test]
    fn practice_base_exp_follows_the_player_level() {
        let base_exps = |hq_level, group| {
            batches(hq_level, group).flatten().map(|rival| calculate_base_exp(&rival))
        };
        for group in GROUPS {
            assert!(base_exps(10, group).all(|exp| (0..=508).contains(&exp)));
        }
        assert!(base_exps(60, RivalType::FirstGroup).all(|exp| (435..=567).contains(&exp)));
        assert!(base_exps(60, RivalType::SecondGroup).all(|exp| (300..=555).contains(&exp)));
    }
}
